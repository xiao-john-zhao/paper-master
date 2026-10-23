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
        .unwrap_or_else(|| format!("未可靠识别{label}；请阅读全文或使用模型分析补充。"))
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
        research_problem: "本地摘录不推断研究问题，请编辑或运行模型分析。".into(),
        research_scenario: if tags.is_empty() {
            "未匹配受管主题词，待人工分类。".into()
        } else {
            format!("题名及摘要命中主题词：{}。", tags.join("、"))
        },
        core_method: "待阅读全文或模型分析后补充方法。".into(),
        key_innovations: vec!["待人工或模型归纳，不根据关键词推断创新。".into()],
        key_contributions: vec!["待人工或模型归纳，不推断贡献。".into()],
        main_results: excerpt(sections.conclusion.as_deref(), "结论"),
        limitations: "本地摘录未评估局限，需阅读全文核实。".into(),
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

#[cfg(test)]
mod tests {
    use super::*;
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
