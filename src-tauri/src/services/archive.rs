use crate::{
    database::{
        analysis_versions,
        paper_repository::{PaperRepository, SqlitePaperRepository},
        Database,
    },
    domain::paper::PaperAnalysisResult,
    error::AppResult,
};

fn excerpt(text: Option<&str>, label: &str) -> String {
    text.filter(|value| !value.trim().is_empty())
        .map(|text| {
            format!(
                "{label}（原文摘录，待人工归纳）：\n{}",
                text.chars().take(1800).collect::<String>()
            )
        })
        .unwrap_or_default()
}
pub fn repair_metadata(database: &Database, id: &str) -> AppResult<()> {
    let connection = database.connection()?;
    let manual: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
            [format!("metadata-edited:{id}")],
            |r| r.get(0),
        )
        .unwrap_or(true);
    if manual {
        return Ok(());
    }
    let repo = SqlitePaperRepository::new(&connection);
    let Some(mut paper) = repo.find_by_id(id)? else {
        return Ok(());
    };
    if !paper.authors.is_empty() && paper.abstract_text.is_some() {
        return Ok(());
    }
    let text = repo.extracted_text(id)?;
    let (sections, _) = crate::pdf::sections::detect_sections(&text);
    let metadata = crate::pdf::parser::extract_metadata(
        std::path::Path::new(&paper.file_path),
        &text,
        sections.abstract_text,
    );
    let mut changed = false;
    if paper.authors.is_empty() && !metadata.authors.is_empty() {
        paper.authors = metadata.authors;
        paper.title = metadata.title;
        changed = true;
    }
    if paper.abstract_text.is_none() && metadata.abstract_text.is_some() {
        paper.abstract_text = metadata.abstract_text;
        changed = true;
    }
    if changed {
        repo.update(&paper)?;
    }
    Ok(())
}

pub fn extract(database: &Database, id: &str) -> AppResult<analysis_versions::AnalysisVersion> {
    let connection = database.connection()?;
    if let Some(existing) = analysis_versions::list(&connection, id)?
        .into_iter()
        .find(|v| v.provider == "extractive" && v.model == "本地原文摘录 v2（非模型）")
    {
        return Ok(existing);
    }
    let repo = SqlitePaperRepository::new(&connection);
    let text = repo.extracted_text(id)?;
    let (sections, _) = crate::pdf::sections::detect_sections(&text);
    let paper = repo.find_by_id(id)?.ok_or_else(|| {
        crate::error::AppError::new(crate::error::ErrorCode::FileNotFound, "论文不存在。")
    })?;
    let evidence = format!(
        "{}\n{}",
        paper.title,
        sections
            .abstract_text
            .as_deref()
            .unwrap_or_else(|| text.get(..text.floor_char_boundary(12000)).unwrap_or(&text))
    );
    let tags = super::keywords::from_evidence(&evidence);
    let result = PaperAnalysisResult {
        schema_version: "2".into(),
        ai_summary: if text.trim().is_empty() {
            "未提取到可读文字；仅建立文件索引，需 OCR 或人工补充。".into()
        } else {
            excerpt(
                sections
                    .abstract_text
                    .as_deref()
                    .or(paper.abstract_text.as_deref()),
                "摘要",
            )
        },
        research_problem: String::new(),
        research_scenario: if tags.is_empty() {
            String::new()
        } else {
            format!("题名及摘要命中主题词：{}。", tags.join("、"))
        },
        core_method: String::new(),
        key_innovations: vec![],
        key_contributions: vec![],
        main_results: excerpt(sections.conclusion.as_deref(), "结论"),
        limitations: String::new(),
        tags,
    };
    let previous = analysis_versions::list(&connection, id)?
        .into_iter()
        .find(|v| v.active);
    let version = analysis_versions::append(
        &connection,
        id,
        "extractive",
        "本地原文摘录 v2（非模型）",
        &result,
    )?;
    if let Some(previous) = previous.filter(|v| v.provider != "extractive") {
        analysis_versions::select(&connection, id, &previous.id)?;
    }
    Ok(version)
}

/// Repair synthetic archive placeholders without rewriting generated model output or manual edits.
pub fn repair_extractive_records(connection: &rusqlite::Connection) -> AppResult<()> {
    use crate::domain::paper::is_extractive_placeholder;
    let repo = SqlitePaperRepository::new(connection);
    for mut paper in repo.list(&Default::default())? {
        let text = repo.extracted_text(&paper.id)?;
        let (sections, _) = crate::pdf::sections::detect_sections(&text);
        for version in analysis_versions::list(connection, &paper.id)?
            .into_iter()
            .filter(|v| v.provider == "extractive" && !v.edited)
        {
            let mut result = version.result.clone();
            for value in [
                &mut result.ai_summary,
                &mut result.research_problem,
                &mut result.research_scenario,
                &mut result.core_method,
                &mut result.main_results,
                &mut result.limitations,
            ] {
                if is_extractive_placeholder(value) {
                    value.clear();
                }
            }
            result
                .key_innovations
                .retain(|v| !is_extractive_placeholder(v));
            result
                .key_contributions
                .retain(|v| !is_extractive_placeholder(v));
            if result.main_results.trim().is_empty() {
                result.main_results = excerpt(sections.conclusion.as_deref(), "结论");
            }
            let json = serde_json::to_string(&result).map_err(|_| {
                crate::error::AppError::new(
                    crate::error::ErrorCode::DatabaseWriteFailed,
                    "摘录格式无效。",
                )
            })?;
            if serde_json::to_string(&version.result).ok().as_deref() != Some(json.as_str()) {
                connection
                    .execute(
                        "UPDATE analysis_versions SET result_json=?1 WHERE id=?2",
                        rusqlite::params![json, version.id],
                    )
                    .map_err(|_| {
                        crate::error::AppError::new(
                            crate::error::ErrorCode::DatabaseWriteFailed,
                            "摘录占位内容修复失败。",
                        )
                    })?;
                if version.active {
                    paper.analysis = Some(result);
                    repo.update(&paper)?;
                }
            }
        }
    }
    connection.execute("UPDATE papers SET ai_analyzed=0,analysis_status=CASE WHEN analysis_status IN ('queued','analyzing','failed') THEN analysis_status ELSE 'not_analyzed' END WHERE id IN (SELECT a.paper_id FROM active_analyses a JOIN analysis_versions v ON v.id=a.version_id WHERE v.provider='extractive') AND (ai_analyzed<>0 OR analysis_status='analyzed')", []).map_err(|_| crate::error::AppError::new(crate::error::ErrorCode::DatabaseWriteFailed, "摘录状态修复失败。"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repair_clears_placeholders_recovers_split_conclusion_and_preserves_manual_versions() {
        let database = Database::in_memory().unwrap();
        let c = database.connection().unwrap();
        c.execute("INSERT INTO papers(id,file_path,file_name,title,extracted_text,created_at,updated_at) VALUES('p','p.pdf','p.pdf','Title','Abstract—Original abstract\nV. C ONCLUSION AND FUTURE WORK\nWe demonstrated a robust method.\nREFERENCES\nCitation must not enter conclusion.','now','now')",[]).unwrap();
        let mut v = extract_result_fixture();
        v.main_results = "未可靠识别结论；请阅读全文或使用模型分析补充。".into();
        let generated = analysis_versions::append(&c, "p", "extractive", "archive", &v).unwrap();
        let manual = analysis_versions::append(&c, "p", "extractive", "manual", &v).unwrap();
        let edited = analysis_versions::edit(&c, "p", &manual.id, 0, &v).unwrap();
        repair_extractive_records(&c).unwrap();
        let versions = analysis_versions::list(&c, "p").unwrap();
        let fixed = versions.iter().find(|v| v.id == generated.id).unwrap();
        assert!(fixed.result.research_problem.is_empty());
        assert!(fixed
            .result
            .main_results
            .contains("We demonstrated a robust method."));
        assert!(!fixed.result.main_results.contains("Citation"));
        let saved = versions.iter().find(|v| v.id == edited.id).unwrap();
        assert_eq!(saved.result.main_results, edited.result.main_results);
        assert_eq!(saved.revision, 1);
        let paper = SqlitePaperRepository::new(&c)
            .find_by_id("p")
            .unwrap()
            .unwrap();
        assert_eq!(paper.analysis_status, "not_analyzed");
        assert!(paper.analysis.is_some());
    }
    fn extract_result_fixture() -> PaperAnalysisResult {
        PaperAnalysisResult {
            schema_version: "2".into(),
            ai_summary: "原文摘录".into(),
            research_problem: "本地摘录不推断研究问题，请编辑或运行模型分析。".into(),
            research_scenario: String::new(),
            core_method: String::new(),
            key_innovations: vec![],
            key_contributions: vec![],
            main_results: String::new(),
            limitations: String::new(),
            tags: vec![],
        }
    }
    #[test]
    fn configured_corpus_archive_is_read_only_and_idempotent() {
        let Ok(corpus) = std::env::var("PAPER_MASTER_TEST_CORPUS") else {
            return;
        };
        let database = Database::in_memory().unwrap();
        let temp = std::env::temp_dir().join(format!("archive-check-{}", uuid::Uuid::new_v4()));
        let logger = crate::diagnostics::AppLogger::new(&temp).unwrap();
        let importer = crate::services::paper_import::PaperImportService::new(&database, &logger);
        let summary = importer.import_paths(vec![corpus.clone()], "archive-validation".into());
        assert!(summary.failed.is_empty());
        assert!(!summary.imported.is_empty());
        for paper in &summary.imported {
            let version = extract(&database, &paper.id).unwrap();
            assert_eq!(extract(&database, &paper.id).unwrap().id, version.id);
            assert_eq!(
                version.result.tags,
                super::super::keywords::normalize(&version.result.tags)
            );
        }
        let duplicate = importer.import_paths(vec![corpus], "archive-repeat".into());
        assert_eq!(duplicate.duplicates.len(), summary.imported.len());
        assert!(duplicate.imported.is_empty());
        assert_eq!(database.integrity_check().unwrap(), "ok");
        eprintln!(
            "ARCHIVE_CHECK imported={} duplicates={} integrity=ok",
            summary.imported.len(),
            duplicate.duplicates.len()
        );
    }
}
