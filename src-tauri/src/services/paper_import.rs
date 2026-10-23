use chrono::Utc;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Instant,
};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    database::{
        paper_repository::{PaperRepository, SqlitePaperRepository},
        settings_storage::{SettingsStorage, SqliteSettingsStorage},
        Database,
    },
    diagnostics::{AppLogger, LogEvent, LogLevel},
    domain::paper::{ImportFailure, ImportSummary, Paper, PdfDiagnostics},
    error::ErrorCode,
    pdf::{
        parser::{fallback_title, PdfDocumentParser},
        DocumentParser,
    },
};

pub struct PaperImportService<'state> {
    database: &'state Database,
    logger: &'state AppLogger,
    parser: PdfDocumentParser,
}

impl<'state> PaperImportService<'state> {
    pub fn new(database: &'state Database, logger: &'state AppLogger) -> Self {
        Self {
            database,
            logger,
            parser: PdfDocumentParser,
        }
    }

    pub fn import_paths(&self, paths: Vec<String>, operation_id: String) -> ImportSummary {
        self.import_observed(paths, operation_id, |_, _, _| {})
    }

    pub fn import_observed(
        &self,
        paths: Vec<String>,
        operation_id: String,
        observer: impl Fn(usize, usize, &str),
    ) -> ImportSummary {
        let started = Instant::now();
        observer(0, 0, "正在扫描 PDF 文件");
        self.record_operation(&operation_id, "running", "Import started");
        let mut start_event = LogEvent::info(
            "paper.import",
            "start",
            format!("Import received {} selected paths", paths.len()),
        );
        start_event.request_id = Some(operation_id.clone());
        self.logger.log(start_event);

        let (candidates, mut failed) = collect_candidates(paths);
        let total = candidates.len();
        let mut imported = Vec::new();
        let mut duplicates = Vec::new();
        let mut seen = HashSet::new();
        let auto_analyze = self
            .database
            .connection()
            .and_then(|connection| SqliteSettingsStorage::new(&connection).get())
            .map(|settings| settings.auto_analyze_on_import)
            .unwrap_or(false);

        for (index, path) in candidates.into_iter().enumerate() {
            observer(index, total, "正在校验路径、提取 PDF 并保存");
            let _progress = FileProgress {
                observer: &observer,
                completed: index + 1,
                total,
            };
            let canonical = match std::fs::canonicalize(&path) {
                Ok(value) => value,
                Err(error) => {
                    failed.push(failure(
                        &path,
                        "FILE_NOT_FOUND",
                        format!("The selected file is unavailable: {error}"),
                    ));
                    continue;
                }
            };
            let path_string = normalize_storage_path(&canonical);
            let canonical = PathBuf::from(&path_string);
            if !seen.insert(path_string.to_lowercase()) {
                duplicates.push(path_string);
                continue;
            }
            let exists = self.database.connection().and_then(|connection| {
                SqlitePaperRepository::new(&connection).exists_by_path(&path_string)
            });
            match exists {
                Ok(true) => {
                    duplicates.push(path_string);
                    continue;
                }
                Err(error) => {
                    failed.push(failure(&canonical, error.code_string(), error.message));
                    continue;
                }
                Ok(false) => {}
            }

            let paper_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            let parsed = self.parser.parse(&canonical);
            let (
                title,
                publication_venue,
                authors,
                year,
                abstract_text,
                extracted_text,
                diagnostics,
            ) = match parsed {
                Ok(document) => (
                    document.metadata.title,
                    document.metadata.publication_venue,
                    document.metadata.authors,
                    document.metadata.year,
                    document.metadata.abstract_text,
                    document.cleaned_text,
                    document.diagnostics,
                ),
                Err(error) => {
                    let mut event = LogEvent::error(
                        "pdf.parser",
                        "extract_text",
                        "PDF extraction failed; a fallback library record will be created",
                    );
                    event.error_code = Some(error.code_string().to_string());
                    event.request_id = Some(operation_id.clone());
                    self.logger.log(event);
                    (
                        fallback_title(&canonical),
                        None,
                        Vec::new(),
                        None,
                        None,
                        String::new(),
                        PdfDiagnostics {
                            page_count: 0,
                            extracted_character_count: 0,
                            detected_sections: Vec::new(),
                            parser_status: "failed".to_string(),
                            warnings: vec![error.message],
                            fallback_used: true,
                        },
                    )
                }
            };
            let paper = Paper {
                id: paper_id,
                file_path: path_string,
                file_name: canonical
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("paper.pdf")
                    .to_string(),
                title,
                publication_venue,
                authors,
                year,
                abstract_text,
                analysis: None,
                tags: Vec::new(),
                analysis_status: if extracted_text.trim().is_empty() {
                    "failed"
                } else if auto_analyze {
                    "queued"
                } else {
                    "not_analyzed"
                }
                .to_string(),
                analysis_error: extracted_text
                    .trim()
                    .is_empty()
                    .then(|| "PDF 未提取到可分析文本；扫描件需要先完成 OCR。".to_string()),
                pdf_diagnostics: Some(diagnostics),
                created_at: now.clone(),
                updated_at: now,
            };
            let save = self.database.connection().and_then(|connection| {
                SqlitePaperRepository::new(&connection).create(&paper, &extracted_text)
            });
            match save {
                Ok(value) => imported.push(value),
                Err(error) if matches!(error.code, ErrorCode::DuplicatePaper) => {
                    duplicates.push(paper.file_path)
                }
                Err(error) => failed.push(failure(&canonical, error.code_string(), error.message)),
            }
        }

        let summary = format!(
            "Imported {}, skipped {} duplicates, failed {}",
            imported.len(),
            duplicates.len(),
            failed.len()
        );
        self.record_operation(
            &operation_id,
            if failed.is_empty() {
                "success"
            } else {
                "partial"
            },
            &summary,
        );
        let mut finish = LogEvent::new(
            if failed.is_empty() {
                LogLevel::Info
            } else {
                LogLevel::Warn
            },
            "paper.import",
            "complete",
            summary,
        );
        finish.request_id = Some(operation_id.clone());
        finish.duration_ms = Some(started.elapsed().as_millis());
        self.logger.log(finish);
        ImportSummary {
            operation_id,
            imported,
            duplicates,
            failed,
        }
    }

    fn record_operation(&self, operation_id: &str, status: &str, summary: &str) {
        if let Ok(connection) = self.database.connection() {
            let _ = connection.execute(
                "INSERT INTO operation_status(operation_id,kind,status,summary,updated_at) VALUES(?1,'import',?2,?3,?4) ON CONFLICT(operation_id) DO UPDATE SET status=excluded.status,summary=excluded.summary,updated_at=excluded.updated_at",
                rusqlite::params![operation_id, status, summary, Utc::now().to_rfc3339()],
            );
        }
    }
}

struct FileProgress<'a> {
    observer: &'a dyn Fn(usize, usize, &str),
    completed: usize,
    total: usize,
}
impl Drop for FileProgress<'_> {
    fn drop(&mut self) {
        (self.observer)(self.completed, self.total, "已处理文件（包含重复或失败项）");
    }
}

fn collect_candidates(paths: Vec<String>) -> (Vec<PathBuf>, Vec<ImportFailure>) {
    let mut candidates = Vec::new();
    let mut failed = Vec::new();
    for raw in paths {
        let path = PathBuf::from(&raw);
        if !path.exists() {
            failed.push(failure(
                &path,
                "FILE_NOT_FOUND",
                "The selected file or folder does not exist.".to_string(),
            ));
            continue;
        }
        if path.is_dir() {
            for entry in WalkDir::new(&path).follow_links(false) {
                match entry {
                    Ok(entry) if entry.file_type().is_file() && is_pdf(entry.path()) => {
                        candidates.push(entry.into_path())
                    }
                    Ok(_) => {}
                    Err(error) => failed.push(failure(
                        &path,
                        "PDF_READ_FAILED",
                        format!("A folder entry could not be read: {error}"),
                    )),
                }
            }
        } else if is_pdf(&path) {
            candidates.push(path);
        } else {
            failed.push(failure(
                &path,
                "PDF_READ_FAILED",
                "Only PDF files can be imported in v0.1.".to_string(),
            ));
        }
    }
    (candidates, failed)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("pdf"))
}

pub(crate) fn normalize_storage_path(path: &Path) -> String {
    let value = path.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = value.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
    }
    value
}

fn failure(path: &Path, code: &str, message: String) -> ImportFailure {
    ImportFailure {
        path: path.display().to_string(),
        code: code.to_string(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database::Database, domain::paper::PaperQuery};

    #[test]
    fn rejects_non_pdf_inputs() {
        let root = std::env::temp_dir().join(format!("paper-master-import-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp directory");
        let text = root.join("notes.txt");
        std::fs::write(&text, "notes").expect("fixture");
        let (candidates, failures) = collect_candidates(vec![text.display().to_string()]);
        assert!(candidates.is_empty());
        assert_eq!(failures[0].code, "PDF_READ_FAILED");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parser_failure_uses_fallback_and_duplicate_is_skipped() {
        let root = std::env::temp_dir().join(format!("paper-master-中文路径-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp directory");
        let pdf = root.join("测试论文.pdf");
        std::fs::write(&pdf, b"%PDF-invalid synthetic fixture").expect("fixture");
        let database = Database::in_memory().expect("database");
        let logger = AppLogger::new(&root).expect("logger");
        let service = PaperImportService::new(&database, &logger);
        let first = service.import_paths(vec![pdf.display().to_string()], "import-test-1".into());
        assert_eq!(first.imported.len(), 1);
        assert_eq!(first.imported[0].title, "测试论文");
        assert!(first.imported[0].file_path.contains("中文路径"));
        assert!(!first.imported[0].file_path.starts_with(r"\\?\"));
        assert_eq!(
            first.imported[0]
                .pdf_diagnostics
                .as_ref()
                .map(|value| value.parser_status.as_str()),
            Some("failed")
        );
        let second = service.import_paths(vec![pdf.display().to_string()], "import-test-2".into());
        assert_eq!(second.duplicates.len(), 1);
        let connection = database.connection().expect("connection");
        assert_eq!(
            SqlitePaperRepository::new(&connection)
                .list(&PaperQuery::default())
                .expect("papers")
                .len(),
            1
        );
        drop(connection);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn progress_counts_fallback_and_duplicate_files_without_changing_sources() {
        let root = std::env::temp_dir().join(format!("paper-master-progress-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("synthetic.pdf");
        let bytes = b"%PDF-invalid synthetic fixture";
        std::fs::write(&source, bytes).unwrap();
        let database = Database::in_memory().unwrap();
        let logger = AppLogger::new(&root).unwrap();
        let events = std::cell::RefCell::new(Vec::new());
        let summary = PaperImportService::new(&database, &logger).import_observed(
            vec![
                source.display().to_string(),
                source.display().to_string(),
                root.join("missing.pdf").display().to_string(),
            ],
            "progress-test".into(),
            |done, total, _| events.borrow_mut().push((done, total)),
        );
        assert_eq!(summary.imported.len(), 1);
        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(
            *events.borrow(),
            vec![(0, 0), (0, 2), (1, 2), (1, 2), (2, 2)]
        );
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        drop(database);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn imports_configured_real_corpus() {
        let Ok(corpus) = std::env::var("PAPER_MASTER_TEST_CORPUS") else {
            return;
        };
        let corpus_path = PathBuf::from(&corpus);
        let expected_pdf_count = WalkDir::new(&corpus_path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file() && is_pdf(entry.path()))
            .count();
        assert!(expected_pdf_count > 0, "configured corpus contains no PDFs");

        let root = std::env::temp_dir().join(format!("paper-master-corpus-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("temp directory");
        let database = Database::in_memory().expect("database");
        let logger = AppLogger::new(&root).expect("logger");
        let service = PaperImportService::new(&database, &logger);
        let summary = service.import_paths(vec![corpus], "import-corpus-smoke".into());
        let fallback_count = summary
            .imported
            .iter()
            .filter(|paper| {
                paper
                    .pdf_diagnostics
                    .as_ref()
                    .is_some_and(|diagnostics| diagnostics.fallback_used)
            })
            .count();

        println!(
            "CORPUS_IMPORT_RESULT expected={} imported={} duplicates={} failed={} fallbacks={}",
            expected_pdf_count,
            summary.imported.len(),
            summary.duplicates.len(),
            summary.failed.len(),
            fallback_count
        );
        assert_eq!(summary.imported.len(), expected_pdf_count);
        assert!(summary.failed.is_empty());
        assert!(summary
            .imported
            .iter()
            .all(|paper| paper.pdf_diagnostics.is_some()));

        let connection = database.connection().expect("connection");
        let repository = SqlitePaperRepository::new(&connection);
        assert_eq!(repository.counts().expect("counts").0, expected_pdf_count);
        let search_result = repository
            .search(&PaperQuery {
                text: Some("communication".into()),
                tag: None,
                filter: None,
            })
            .expect("search");
        println!(
            "CORPUS_SEARCH_RESULT query=communication matches={}",
            search_result.len()
        );
        assert!(!search_result.is_empty());
        drop(connection);

        let duplicate_summary = service.import_paths(
            vec![corpus_path.display().to_string()],
            "import-corpus-duplicate-smoke".into(),
        );
        assert_eq!(duplicate_summary.duplicates.len(), expected_pdf_count);
        assert!(duplicate_summary.imported.is_empty());

        let source = summary
            .imported
            .iter()
            .find(|paper| {
                paper
                    .pdf_diagnostics
                    .as_ref()
                    .is_some_and(|diagnostics| !diagnostics.fallback_used)
            })
            .expect("at least one extractable PDF");
        let chinese_directory = root.join("中文文献目录");
        std::fs::create_dir_all(&chinese_directory).expect("Chinese directory");
        let chinese_pdf = chinese_directory.join("中文路径测试论文.pdf");
        std::fs::copy(&source.file_path, &chinese_pdf).expect("copy PDF fixture");
        let chinese_summary = service.import_paths(
            vec![chinese_pdf.display().to_string()],
            "import-chinese-path-smoke".into(),
        );
        assert_eq!(chinese_summary.imported.len(), 1);
        assert!(chinese_summary.failed.is_empty());
        assert!(chinese_summary.imported[0]
            .file_path
            .contains("中文文献目录"));
        assert!(!chinese_summary.imported[0].file_path.starts_with(r"\\?\"));
        assert!(chinese_summary.imported[0]
            .pdf_diagnostics
            .as_ref()
            .is_some_and(|diagnostics| diagnostics.extracted_character_count > 0));
        println!(
            "CHINESE_PATH_RESULT imported=1 extracted_characters={}",
            chinese_summary.imported[0]
                .pdf_diagnostics
                .as_ref()
                .map_or(0, |diagnostics| diagnostics.extracted_character_count)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn first_import_is_queued_once_and_automatic_analysis_can_be_disabled() {
        let root =
            std::env::temp_dir().join(format!("paper-master-auto-import-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let stream = "BT /F1 12 Tf 50 700 Td (A paper about wireless scheduling and fair resource allocation.) Tj ET";
        let objects = ["<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".into(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into(),
            format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len())];
        let mut document = "%PDF-1.4\n".to_string();
        let mut offsets = vec![0];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(document.len());
            document.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
        }
        let xref = document.len();
        document.push_str("xref\n0 6\n0000000000 65535 f \n");
        for offset in offsets.iter().skip(1) {
            document.push_str(&format!("{offset:010} 00000 n \n"));
        }
        document.push_str(&format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
        ));
        let source = root.join("first.pdf");
        std::fs::write(&source, &document).unwrap();
        let database = Database::in_memory().unwrap();
        let logger = AppLogger::new(&root).unwrap();
        let importer = PaperImportService::new(&database, &logger);
        let imported = importer.import_paths(vec![source.display().to_string()], "first".into());
        assert_eq!(imported.imported[0].analysis_status, "queued");
        let duplicate =
            importer.import_paths(vec![source.display().to_string()], "duplicate".into());
        assert!(duplicate.imported.is_empty());
        assert_eq!(duplicate.duplicates.len(), 1);
        assert_eq!(std::fs::read_to_string(&source).unwrap(), document);
        {
            let connection = database.connection().unwrap();
            SqliteSettingsStorage::new(&connection)
                .save(&crate::domain::settings::AppSettings {
                    auto_analyze_on_import: false,
                    ..Default::default()
                })
                .unwrap();
        }
        let disabled = root.join("disabled.pdf");
        std::fs::write(&disabled, &document).unwrap();
        let imported =
            importer.import_paths(vec![disabled.display().to_string()], "disabled".into());
        assert_eq!(imported.imported[0].analysis_status, "not_analyzed");
    }
}
