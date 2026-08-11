use crate::{
    domain::paper::Paper,
    error::{AppError, AppResult, ErrorCode},
    pdf::sections::detect_sections,
};

pub struct PaperAnalysisInputBuilder {
    max_characters: usize,
}

impl PaperAnalysisInputBuilder {
    pub fn new(max_characters: usize) -> Self {
        Self { max_characters }
    }

    pub fn build(&self, paper: &Paper, extracted_text: &str) -> AppResult<String> {
        if extracted_text.trim().is_empty() {
            return Err(AppError::new(ErrorCode::PdfTextEmpty, "No embedded text is available for AI analysis. The PDF may be scanned or image-only."));
        }
        let (sections, _) = detect_sections(extracted_text);
        let mut output = String::with_capacity(self.max_characters.min(extracted_text.len()));
        push_section(
            &mut output,
            "TITLE",
            &paper.title,
            500.min(self.max_characters / 10),
            self.max_characters,
        );
        if let Some(abstract_text) = paper.abstract_value().or(sections.abstract_text.as_deref()) {
            push_section(
                &mut output,
                "ABSTRACT",
                abstract_text,
                self.max_characters * 20 / 100,
                self.max_characters,
            );
        }
        if let Some(introduction) = sections.introduction.as_deref() {
            push_section(
                &mut output,
                "INTRODUCTION",
                introduction,
                self.max_characters * 30 / 100,
                self.max_characters,
            );
        }
        if let Some(conclusion) = sections.conclusion.as_deref() {
            push_section(
                &mut output,
                "CONCLUSION",
                conclusion,
                self.max_characters * 20 / 100,
                self.max_characters,
            );
        }
        if output.chars().count() < self.max_characters {
            push_section(
                &mut output,
                "SELECTED BODY",
                extracted_text,
                self.max_characters,
                self.max_characters,
            );
        }
        Ok(output.chars().take(self.max_characters).collect())
    }
}

fn push_section(
    output: &mut String,
    heading: &str,
    value: &str,
    section_budget: usize,
    absolute_limit: usize,
) {
    let current = output.chars().count();
    if current >= absolute_limit {
        return;
    }
    let prefix = format!("\n\n[{heading}]\n");
    let remaining = absolute_limit.saturating_sub(current);
    if remaining <= prefix.chars().count() {
        return;
    }
    output.push_str(&prefix);
    output.extend(
        value
            .chars()
            .take((remaining - prefix.chars().count()).min(section_budget)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn paper() -> Paper {
        Paper {
            id: "1".into(),
            file_path: "x.pdf".into(),
            file_name: "x.pdf".into(),
            title: "Bounded Local Analysis".into(),
            authors: vec![],
            year: None,
            abstract_text: Some("Stored abstract".into()),
            analysis: None,
            tags: vec![],
            analysis_status: "not_analyzed".into(),
            analysis_error: None,
            pdf_diagnostics: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn input_is_bounded_and_prioritizes_sections() {
        let text = format!(
            "ABSTRACT\nA short abstract.\nINTRODUCTION\n{}\nCONCLUSION\nFinal evidence.",
            "body ".repeat(5_000)
        );
        let result = PaperAnalysisInputBuilder::new(1_200)
            .build(&paper(), &text)
            .expect("input");
        assert!(result.chars().count() <= 1_200);
        assert!(result.contains("[TITLE]"));
        assert!(result.contains("[ABSTRACT]"));
        assert!(result.contains("[CONCLUSION]"));
    }

    #[test]
    fn empty_text_is_explicit_error() {
        let error = PaperAnalysisInputBuilder::new(1_000)
            .build(&paper(), "  ")
            .expect_err("empty text");
        assert_eq!(error.code_string(), "PDF_TEXT_EMPTY");
    }
}
