use super::DocumentSections;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionKind {
    Abstract,
    Introduction,
    Conclusion,
}

pub fn detect_sections(text: &str) -> (DocumentSections, Vec<String>) {
    let lines: Vec<&str> = text.lines().collect();
    let mut markers: Vec<(usize, SectionKind, String)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if let Some((kind, inline)) = classify_heading(line) {
            markers.push((index, kind, inline));
        }
    }

    let mut sections = DocumentSections::default();
    let mut detected = Vec::new();
    for (marker_index, (line_index, kind, inline)) in markers.iter().enumerate() {
        let end = markers
            .get(marker_index + 1)
            .map(|value| value.0)
            .unwrap_or(lines.len());
        let mut content = Vec::new();
        if !inline.is_empty() {
            content.push(inline.as_str());
        }
        content.extend(lines[line_index + 1..end].iter().copied());
        let value = content.join("\n").trim().to_string();
        if value.is_empty() {
            continue;
        }
        match kind {
            SectionKind::Abstract if sections.abstract_text.is_none() => {
                sections.abstract_text = Some(value);
                detected.push("abstract".to_string());
            }
            SectionKind::Introduction if sections.introduction.is_none() => {
                sections.introduction = Some(value);
                detected.push("introduction".to_string());
            }
            SectionKind::Conclusion if sections.conclusion.is_none() => {
                sections.conclusion = Some(value);
                detected.push("conclusion".to_string());
            }
            _ => {}
        }
    }
    (sections, detected)
}

fn classify_heading(line: &str) -> Option<(SectionKind, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.len() > 160 {
        return None;
    }
    let lowercase = trimmed.to_lowercase();
    for (label, kind) in [
        ("abstract", SectionKind::Abstract),
        ("introduction", SectionKind::Introduction),
        ("conclusions", SectionKind::Conclusion),
        ("conclusion", SectionKind::Conclusion),
    ] {
        if lowercase == label {
            return Some((kind, String::new()));
        }
        if let Some(rest) = lowercase.strip_prefix(label) {
            if rest.starts_with(['—', '–', '-', ':']) {
                let inline = trimmed[label.len()..]
                    .trim_start_matches(['—', '–', '-', ':', ' '])
                    .to_string();
                return Some((kind, inline));
            }
        }
        if lowercase.ends_with(label) {
            let prefix = lowercase[..lowercase.len() - label.len()].trim();
            if !prefix.is_empty()
                && prefix.chars().all(|value| {
                    value.is_ascii_digit()
                        || value.is_ascii_whitespace()
                        || matches!(value, '.' | ':' | 'i' | 'v' | 'x')
                })
            {
                return Some((kind, String::new()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ieee_and_inline_headings_case_insensitively() {
        let text = "A Paper\nABSTRACT—This is the abstract.\nI. INTRODUCTION\nIntro body.\nV. Conclusions\nFinal body.";
        let (sections, detected) = detect_sections(text);
        assert_eq!(
            sections.abstract_text.as_deref(),
            Some("This is the abstract.")
        );
        assert_eq!(sections.introduction.as_deref(), Some("Intro body."));
        assert_eq!(sections.conclusion.as_deref(), Some("Final body."));
        assert_eq!(detected, vec!["abstract", "introduction", "conclusion"]);
    }

    #[test]
    fn avoids_paragraph_mentions_as_headings() {
        let (sections, _) = detect_sections("The introduction explains our method in detail.");
        assert!(sections.introduction.is_none());
    }
}
