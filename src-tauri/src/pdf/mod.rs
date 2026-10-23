pub mod parser;
pub mod poppler;
pub mod sections;

use std::path::Path;

use serde::Serialize;

use crate::{domain::paper::PdfDiagnostics, error::AppResult};

#[derive(Debug, Clone, Default, Serialize)]
pub struct DocumentMetadata {
    pub title: String,
    pub publication_venue: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub abstract_text: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentSections {
    pub abstract_text: Option<String>,
    pub introduction: Option<String>,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedDocument {
    pub cleaned_text: String,
    pub sections: DocumentSections,
    pub metadata: DocumentMetadata,
    pub diagnostics: PdfDiagnostics,
}

pub trait DocumentParser: Send + Sync {
    fn supports(&self, path: &Path) -> bool;
    fn parse(&self, path: &Path) -> AppResult<ParsedDocument>;
}
