use regex::Regex;
use std::path::Path;

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
        let pages = pdf_extract::extract_text_by_pages(path).map_err(|error| {
            AppError::new(
                ErrorCode::PdfReadFailed,
                "Text could not be extracted from this PDF.",
            )
            .detail(error.to_string())
        })?;
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

fn extract_metadata(path: &Path, text: &str, abstract_text: Option<String>) -> DocumentMetadata {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(20)
        .collect();
    let title = lines
        .iter()
        .find(|line| plausible_title(line))
        .map(|line| (*line).to_string())
        .unwrap_or_else(|| fallback_title(path));
    let title_index = lines.iter().position(|line| *line == title).unwrap_or(0);
    let authors = lines
        .get(title_index + 1)
        .filter(|line| plausible_author_line(line))
        .map(|line| split_authors(line))
        .unwrap_or_default();
    DocumentMetadata {
        title,
        authors,
        year: detect_year(&text.chars().take(4_000).collect::<String>()),
        abstract_text,
    }
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

fn split_authors(line: &str) -> Vec<String> {
    line.replace(" and ", ",")
        .replace(';', ",")
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
        .filter(|value| value.len() >= 2 && value.len() <= 80)
        .take(12)
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
}
