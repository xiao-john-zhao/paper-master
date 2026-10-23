use regex::Regex;
use std::{panic::AssertUnwindSafe, path::Path};

use crate::{
    domain::paper::PdfDiagnostics,
    error::{AppError, AppResult, ErrorCode},
};

use super::{sections::detect_sections, DocumentMetadata, DocumentParser, ParsedDocument};

#[derive(Default)]
pub struct PdfDocumentParser;

impl DocumentParser for PdfDocumentParser {
    fn supports(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("pdf"))
    }

    fn parse(&self, path: &Path) -> AppResult<ParsedDocument> {
        if !path.exists() {
            return Err(AppError::new(
                ErrorCode::FileNotFound,
                "The selected PDF no longer exists.",
            )
            .detail(path.display().to_string()));
        }
        let pages = if let Some(pages) = super::poppler::extract(path) {
            pages
        } else {
            std::panic::catch_unwind(AssertUnwindSafe(|| {
            pdf_extract::extract_text_by_pages(path)
        }))
        .map_err(|_| {
            AppError::new(
                ErrorCode::PdfReadFailed,
                "The PDF parser could not safely read this document; a fallback record will be used.",
            )
            .detail("A third-party PDF parser panic was isolated at the document boundary.")
        })?
        .map_err(|error| {
            AppError::new(
                ErrorCode::PdfReadFailed,
                "Text could not be extracted from this PDF.",
            )
            .detail(error.to_string())
        })?
        };
        let cleaned_text = clean_text(&pages.join("\n\n"));
        let (sections, detected_sections) = detect_sections(&cleaned_text);
        let metadata = extract_metadata(path, &cleaned_text, sections.abstract_text.clone());
        let mut warnings = Vec::new();
        if cleaned_text.trim().is_empty() {
            warnings.push(
                "No embedded text was found; the PDF may be scanned or image-only.".to_string(),
            );
        }
        if sections.abstract_text.is_none() {
            warnings.push("Abstract section was not detected.".to_string());
        }
        if sections.introduction.is_none() {
            warnings.push("Introduction section was not detected.".to_string());
        }
        if sections.conclusion.is_none() {
            warnings.push("Conclusion section was not detected.".to_string());
        }
        let fallback_used = detected_sections.len() < 2;
        Ok(ParsedDocument {
            diagnostics: PdfDiagnostics {
                page_count: pages.len(),
                extracted_character_count: cleaned_text.chars().count(),
                detected_sections,
                parser_status: if cleaned_text.is_empty() {
                    "failed"
                } else if warnings.len() > 1 {
                    "partial"
                } else {
                    "success"
                }
                .to_string(),
                warnings,
                fallback_used,
            },
            cleaned_text,
            sections,
            metadata,
        })
    }
}

pub fn clean_text(input: &str) -> String {
    let normalized = input
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{0000}', "");
    let hyphenated = Regex::new(r"(?m)([A-Za-z])-[ \t]*\n[ \t]*([a-z])")
        .expect("static regex")
        .replace_all(&normalized, "$1$2");
    let spaces = Regex::new(r"[ \t]+")
        .expect("static regex")
        .replace_all(&hyphenated, " ");
    let excess_lines = Regex::new(r"\n{3,}")
        .expect("static regex")
        .replace_all(&spaces, "\n\n");
    excess_lines
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub(crate) fn extract_metadata(
    path: &Path,
    text: &str,
    abstract_text: Option<String>,
) -> DocumentMetadata {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(40)
        .collect();
    let (title, _, title_end_index) = detect_title(path, &lines);
    let author_block = lines
        .iter()
        .skip(title_end_index + 1)
        .take(16)
        .take_while(|line| !is_author_block_boundary(line))
        .filter(|line| plausible_author_line(line))
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    let mut authors = split_authors(&author_block);
    let mut title = title;
    // Degree theses often put the institution on the cover and the author in the filename.
    // Accept that name only when it also occurs in the actual cover text.
    let cover: String = text.chars().take(8000).collect();
    if cover.contains("学位论文") || cover.contains("MASTER THESIS") {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if let Some((name_title, name)) = stem.rsplit_once('_') {
            if (2..=4).contains(&name.chars().count())
                && name.chars().all(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
                && cover.split_whitespace().collect::<String>().contains(name)
            {
                authors = vec![name.into()];
                title = name_title.replace('_', " ");
            }
        }
    }
    DocumentMetadata {
        title,
        publication_venue: publication_venue_from_text(text),
        authors,
        year: detect_year(&text.chars().take(4_000).collect::<String>()),
        abstract_text,
    }
}

pub(crate) fn title_from_text(path: &Path, text: &str) -> String {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(40)
        .collect::<Vec<_>>();
    detect_title(path, &lines).0
}

fn detect_title(path: &Path, lines: &[&str]) -> (String, usize, usize) {
    let Some(start_index) = lines.iter().position(|line| {
        plausible_title(line)
            && !looks_like_publication_header(line)
            && !looks_like_affiliation_or_author(line)
    }) else {
        return (fallback_title(path), 0, 0);
    };

    let filename_words = normalized_words(
        &path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
    );
    let filename_is_descriptive = filename_words.len() >= 3;
    let mut title_parts = vec![lines[start_index]];
    let mut end_index = start_index;

    for (index, line) in lines.iter().enumerate().skip(start_index + 1).take(4) {
        if !plausible_title(line)
            || looks_like_publication_header(line)
            || looks_like_affiliation_or_author(line)
        {
            break;
        }

        if filename_is_descriptive {
            let current_coverage = filename_coverage(&title_parts.join(" "), &filename_words);
            let next_coverage = filename_coverage(line, &filename_words);
            if current_coverage >= 0.55 && next_coverage == 0.0 {
                break;
            }
        }

        let combined_length = title_parts.iter().map(|part| part.len()).sum::<usize>()
            + title_parts.len()
            + line.len();
        if combined_length > 320 {
            break;
        }
        title_parts.push(line);
        end_index = index;
    }

    (title_parts.join(" "), start_index, end_index)
}

fn normalized_words(value: &str) -> Vec<String> {
    Regex::new(r"[^\p{L}\p{N}]+")
        .expect("static regex")
        .split(&value.to_lowercase())
        .filter(|word| word.chars().count() >= 3 && !word.chars().all(char::is_numeric))
        .map(str::to_string)
        .collect()
}

fn filename_coverage(value: &str, filename_words: &[String]) -> f32 {
    if filename_words.is_empty() {
        return 0.0;
    }
    let candidate_words = normalized_words(value);
    let matches = filename_words
        .iter()
        .filter(|word| candidate_words.contains(word))
        .count();
    matches as f32 / filename_words.len() as f32
}

fn looks_like_publication_header(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("vol.")
        || lower.contains("volume ")
        || lower.contains(" no. ")
        || lower.contains("issue ")
        || lower.contains("issn")
        || lower.contains("copyright")
        || lower.starts_with("arxiv:")
        || lower.starts_with("chinese journal of")
        || lower.starts_with("ieee transactions on")
        || matches!(
            lower.as_str(),
            "tsinghua science and technology"
                | "research article"
                | "review article"
                | "open access"
                | "article"
        )
        || Regex::new(r"(?i)^\s*\d{1,6}\s+(?:ieee|acm|springer|elsevier)\b")
            .expect("static regex")
            .is_match(line)
}

fn looks_like_affiliation_or_author(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("abstract")
        || lower.starts_with("keywords")
        || lower.starts_with("index terms")
        || lower.starts_with("doi")
        || lower.contains('@')
        || lower.contains("university")
        || lower.contains("department of")
        || lower.contains("school of")
        || lower.contains("institute of")
        || lower.contains("member, ieee")
        || lower.contains("fellow, ieee")
        || line.matches(',').count() >= 2
}

pub(crate) fn publication_venue_from_text(text: &str) -> Option<String> {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(40)
        .collect::<Vec<_>>();
    detect_publication_venue(&lines)
}

fn detect_publication_venue(lines: &[&str]) -> Option<String> {
    let venue_signal = Regex::new(
        r"(?i)(journal|proceedings|conference|symposium|workshop|transactions|letters|review|bulletin|annals|archives|arxiv|preprint|学报|期刊|会议|研讨会|论坛|出版社|杂志)",
    )
    .expect("static regex");
    let named_venue =
        Regex::new(r"(?i)^(nature(?:\s+\w+){0,4}|science|cell(?:\s+\w+){0,3}|plos\s+\w+)\b")
            .expect("static regex");
    let journal_citation =
        Regex::new(r"(?i)^.{3,120}\s+\d{1,4}\s*\((?:19|20)\d{2}\)").expect("static regex");

    let mut candidates = lines.iter().take(24).filter(|line| {
        let lower = line.to_lowercase();
        !lower.starts_with("abstract")
            && !lower.starts_with("keywords")
            && !lower.starts_with("doi")
            && !lower.starts_with("received")
            && !lower.starts_with("accepted")
            && !lower.contains('@')
            && !lower.contains("university")
            && !lower.contains("department of")
            && !lower.contains("school of")
            && !lower.contains("institute of")
    });

    candidates
        .clone()
        .find_map(|line| {
            (looks_like_publication_header(line)
                && (venue_signal.is_match(line)
                    || named_venue.is_match(line)
                    || journal_citation.is_match(line)))
            .then(|| simplify_publication_venue(line))
            .flatten()
        })
        .or_else(|| {
            candidates.find_map(|line| {
                (venue_signal.is_match(line)
                    || named_venue.is_match(line)
                    || journal_citation.is_match(line))
                .then(|| simplify_publication_venue(line))
                .flatten()
            })
        })
}

fn simplify_publication_venue(line: &str) -> Option<String> {
    let without_page = Regex::new(r"^\s*\d{1,6}\s+")
        .expect("static regex")
        .replace(line, "");
    let without_prefix = Regex::new(r"(?i)^\s*(?:published\s+in|in)\s*:?\s*")
        .expect("static regex")
        .replace(&without_page, "");
    let without_details = Regex::new(
        r"(?i)(?:\s*[,;|]\s*(?:vol(?:ume)?\.?|no\.?|issue|pp?\.?|pages?|doi)\b|\s+\d{1,4}\s*\((?:19|20)\d{2}\)).*$",
    )
    .expect("static regex")
    .replace(&without_prefix, "");
    let without_year = Regex::new(r"\b(?:19|20)\d{2}\b")
        .expect("static regex")
        .replace_all(&without_details, " ");
    let normalized = Regex::new(r"\s+")
        .expect("static regex")
        .replace_all(&without_year, " ")
        .trim_matches(|character: char| {
            character.is_whitespace() || matches!(character, ',' | ';' | '|' | '-' | '.')
        })
        .to_string();
    (normalized.len() >= 3 && normalized.len() <= 180).then_some(normalized)
}

fn plausible_title(line: &str) -> bool {
    let lower = line.to_lowercase();
    (8..=240).contains(&line.len())
        && !matches!(
            lower.as_str(),
            "abstract" | "introduction" | "conclusion" | "conclusions"
        )
        && !lower.contains("doi:")
        && !lower.contains("http://")
        && !lower.contains("https://")
}

fn plausible_author_line(line: &str) -> bool {
    (3..=260).contains(&line.len())
        && !line.contains('@')
        && !line.to_lowercase().contains("university")
        && !line.chars().all(|character| {
            character.is_ascii_uppercase()
                || character.is_ascii_whitespace()
                || character.is_ascii_punctuation()
        })
}

fn is_author_block_boundary(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("abstract")
        || lower.starts_with("keywords")
        || lower.starts_with("index terms")
        || lower.starts_with("received")
        || lower.starts_with("accepted")
        || lower.starts_with("doi")
        || lower.contains('@')
        || lower.contains("university")
        || lower.contains("department of")
        || lower.contains("school of")
        || lower.contains("institute of")
        || looks_like_publication_header(line)
        || publication_venue_from_text(line).is_some()
}

fn split_authors(line: &str) -> Vec<String> {
    let without_roles = Regex::new(
        r"(?i)\b(?:(?:senior|graduate student|student|life)\s+)*(?:member|fellow)\b|\bieee\b",
    )
    .expect("static regex")
    .replace_all(line, "");
    without_roles
        .replace(" and ", ",")
        .replace(';', ",")
        .replace(['，', '、'], ",")
        .split(',')
        .map(|value| {
            value
                .trim()
                .trim_matches(|character: char| {
                    character.is_ascii_digit() || matches!(character, '*' | '†' | '‡')
                })
                .trim()
                .to_string()
        })
        .filter(|value| {
            value.len() >= 2
                && value.len() <= 80
                && !matches!(
                    value.to_lowercase().as_str(),
                    "member"
                        | "senior member"
                        | "student member"
                        | "fellow"
                        | "ieee"
                        | "life"
                        | "senior"
                        | "student"
                )
        })
        .take(100)
        .collect()
}

pub(crate) fn clean_author_roles(authors: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    authors
        .iter()
        .flat_map(|name| split_authors(name))
        .filter(|name| seen.insert(name.to_lowercase()))
        .collect()
}

fn detect_year(text: &str) -> Option<i32> {
    Regex::new(r"\b(?:19|20)\d{2}\b")
        .expect("static regex")
        .find_iter(text)
        .filter_map(|value| value.as_str().parse::<i32>().ok())
        .find(|year| {
            (1900..=chrono::Utc::now()
                .format("%Y")
                .to_string()
                .parse::<i32>()
                .unwrap_or(2100)
                + 1)
                .contains(year)
        })
}

pub fn fallback_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Untitled paper")
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn life_fellow_and_life_senior_member_are_roles_not_authors() {
        let authors =
            split_authors("Ada Lovelace, Life Fellow, IEEE, Alan Turing, Life Senior Member, IEEE");
        assert_eq!(authors, ["Ada Lovelace", "Alan Turing"]);
        assert_eq!(
            clean_author_roles(&["Ada Lovelace".into(), "Life".into(), "Alan Turing".into()]),
            ["Ada Lovelace", "Alan Turing"]
        );
    }

    #[test]
    fn skips_publisher_cover_and_retains_real_author_block() {
        let text="Chinese Journal of Electronics\nvol. 33, no. 6\nhttps://doi.org/10.123/example\nRESEARCH ARTICLE\nPerformance Analysis of Spatial Modulation\nAided UAV Communication Systems\nXiangbin YU, Ning LI, and Cuimin PAN\nAbstract: We study communication.\n";
        let meta = extract_metadata(
            Path::new(
                "Performance_Analysis_of_Spatial_Modulation_Aided_UAV_Communication_Systems.pdf",
            ),
            text,
            None,
        );
        assert!(meta.title.starts_with("Performance Analysis"));
        assert_eq!(meta.authors.len(), 3);
    }

    #[test]
    fn cleaning_joins_hyphenated_words_and_limits_blank_lines() {
        assert_eq!(
            clean_text("scien-\ntific\n\n\n\ntext"),
            "scientific\n\ntext"
        );
    }

    #[test]
    fn title_fallback_is_readable() {
        assert_eq!(
            fallback_title(Path::new("C:\\papers\\local_model-study.pdf")),
            "local model study"
        );
    }

    #[test]
    fn finds_recent_year() {
        assert_eq!(detect_year("Published in 2025."), Some(2025));
    }

    #[test]
    fn extracts_and_simplifies_journal_or_conference_venue() {
        let journal = extract_metadata(
            Path::new("paper.pdf"),
            "A Reliable Paper Title\nAda Lovelace, Alan Turing\nIEEE Transactions on Knowledge and Data Engineering, Vol. 36, No. 4, 2024\nAbstract\nText",
            None,
        );
        assert_eq!(
            journal.publication_venue.as_deref(),
            Some("IEEE Transactions on Knowledge and Data Engineering")
        );

        let conference = extract_metadata(
            Path::new("paper.pdf"),
            "A Reliable Paper Title\nAda Lovelace\nProceedings of the 2025 ACM SIGIR Conference\nAbstract\nText",
            None,
        );
        assert_eq!(
            conference.publication_venue.as_deref(),
            Some("Proceedings of the ACM SIGIR Conference")
        );
    }

    #[test]
    fn extracts_multiline_title_instead_of_running_journal_header() {
        let metadata = extract_metadata(
            Path::new("Fair Integrated Sensing and Communication for Multi-UAV-Enabled Internet of Things Joint 3-D Trajectory and Resource Optimization.pdf"),
            "29546 IEEE INTERNET OF THINGS JOURNAL, VOL. 11, NO. 18, 15 SEPTEMBER 2024\nFair Integrated Sensing and Communication for\nMulti-UAV-Enabled Internet of Things: Joint 3-D\nTrajectory and Resource Optimization\nXin Liu, Senior Member, IEEE, Yuemin Liu, Zechen Liu, and Tariq S. Durrani\nAbstract—Unmanned aerial vehicles are studied.",
            None,
        );

        assert_eq!(
            metadata.title,
            "Fair Integrated Sensing and Communication for Multi-UAV-Enabled Internet of Things: Joint 3-D Trajectory and Resource Optimization"
        );
        assert_eq!(
            metadata.publication_venue.as_deref(),
            Some("IEEE INTERNET OF THINGS JOURNAL")
        );
        assert_eq!(
            metadata.authors,
            vec!["Xin Liu", "Yuemin Liu", "Zechen Liu", "Tariq S. Durrani"]
        );
    }
}
