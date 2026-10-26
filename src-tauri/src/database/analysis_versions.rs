use super::paper_repository::{PaperRepository, SqlitePaperRepository};
use crate::{
    domain::paper::PaperAnalysisResult,
    error::{AppError, AppResult, ErrorCode},
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisVersion {
    pub id: String,
    pub paper_id: String,
    pub provider: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
    pub revision: i64,
    pub edited: bool,
    pub active: bool,
    pub result: PaperAnalysisResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<crate::ai::adaptive::AnalysisProfile>,
}
fn failure() -> AppError {
    AppError::new(
        ErrorCode::DatabaseWriteFailed,
        "分析结果无法保存或读取，请刷新后重试。",
    )
}
pub fn list(connection: &Connection, paper_id: &str) -> AppResult<Vec<AnalysisVersion>> {
    let mut query = connection.prepare("SELECT v.id,v.paper_id,v.provider,v.model,v.created_at,v.updated_at,v.revision,v.edited,v.result_json,COALESCE(a.version_id=v.id,0) FROM analysis_versions v LEFT JOIN active_analyses a ON a.paper_id=v.paper_id WHERE v.paper_id=?1 ORDER BY v.created_at DESC,v.id DESC").map_err(|_| failure())?;
    let rows = query
        .query_map([paper_id], |row| {
            let json: String = row.get(8)?;
            let result = serde_json::from_str(&json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok(AnalysisVersion {
                id: row.get(0)?,
                paper_id: row.get(1)?,
                provider: row.get(2)?,
                model: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: row.get(6)?,
                edited: row.get(7)?,
                result,
                active: row.get(9)?,
                profile: None,
            })
        })
        .map_err(|_| failure())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| failure())?;
    let mut rows = rows;
    for row in &mut rows {
        let json: Option<String> = connection
            .query_row(
                "SELECT value_json FROM settings WHERE key=?1",
                [format!("analysis-profile:{}", row.id)],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| failure())?;
        row.profile = json.and_then(|s| serde_json::from_str(&s).ok());
    }
    Ok(rows)
}

// Savepoints make selecting/editing a version and its search/list projection atomic,
// including when called inside a schema migration transaction.
fn atomic<T>(connection: &Connection, action: impl FnOnce() -> AppResult<T>) -> AppResult<T> {
    connection
        .execute_batch("SAVEPOINT analysis_change")
        .map_err(|_| failure())?;
    match action() {
        Ok(value) => {
            connection
                .execute_batch("RELEASE analysis_change")
                .map_err(|_| failure())?;
            Ok(value)
        }
        Err(error) => {
            let _ =
                connection.execute_batch("ROLLBACK TO analysis_change; RELEASE analysis_change");
            Err(error)
        }
    }
}
fn project(connection: &Connection, version: &AnalysisVersion) -> AppResult<()> {
    let repo = SqlitePaperRepository::new(connection);
    let mut paper = repo.find_by_id(&version.paper_id)?.ok_or_else(failure)?;
    paper.analysis = Some(version.result.clone());
    paper.tags = version.result.tags.clone();
    paper.analysis_provider = Some(version.provider.clone());
    paper.analysis_status = if version.provider == "extractive" {
        "not_analyzed"
    } else {
        "analyzed"
    }
    .into();
    paper.analysis_error = None;
    paper.updated_at = Utc::now().to_rfc3339();
    repo.update(&paper)?;
    connection.execute("INSERT INTO active_analyses(paper_id,version_id) VALUES(?1,?2) ON CONFLICT(paper_id) DO UPDATE SET version_id=excluded.version_id",params![version.paper_id,version.id]).map_err(|_| failure())?;
    Ok(())
}
pub fn append(
    connection: &Connection,
    paper_id: &str,
    provider: &str,
    model: &str,
    result: &PaperAnalysisResult,
) -> AppResult<AnalysisVersion> {
    let mut result = result.clone();
    result.tags = crate::services::keywords::normalize(&result.tags);
    let now = Utc::now().to_rfc3339();
    let version = AnalysisVersion {
        id: uuid::Uuid::new_v4().to_string(),
        paper_id: paper_id.into(),
        provider: provider.into(),
        model: model.into(),
        created_at: now.clone(),
        updated_at: now,
        revision: 0,
        edited: false,
        active: true,
        profile: None,
        result,
    };
    atomic(connection, || {
        connection.execute("INSERT INTO analysis_versions(id,paper_id,provider,model,created_at,updated_at,revision,edited,result_json) VALUES(?1,?2,?3,?4,?5,?6,0,0,?7)",params![version.id,version.paper_id,version.provider,version.model,version.created_at,version.updated_at,serde_json::to_string(&version.result).map_err(|_| failure())?]).map_err(|_| failure())?;
        project(connection, &version)?;
        Ok(version.clone())
    })
}
pub fn select(connection: &Connection, paper_id: &str, id: &str) -> AppResult<AnalysisVersion> {
    atomic(connection, || {
        let version = list(connection, paper_id)?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(failure)?;
        project(connection, &version)?;
        Ok(AnalysisVersion {
            active: true,
            ..version
        })
    })
}
pub fn edit(
    connection: &Connection,
    paper_id: &str,
    id: &str,
    revision: i64,
    result: &PaperAnalysisResult,
) -> AppResult<AnalysisVersion> {
    if result.schema_version != "1" && result.schema_version != "2" {
        return Err(failure());
    }
    let encoded = serde_json::to_string(result).map_err(|_| failure())?;
    if encoded.len() > 200_000 || result.tags.len() > 100 {
        return Err(AppError::new(
            ErrorCode::AiSchemaValidationFailed,
            "编辑内容过长。",
        ));
    }
    let mut version = list(connection, paper_id)?
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(failure)?;
    version.result = result.clone();
    version.result.tags = crate::services::keywords::normalize(&result.tags);
    version.updated_at = Utc::now().to_rfc3339();
    version.edited = true;
    atomic(connection, || {
        let changed = connection.execute("UPDATE analysis_versions SET original_json=COALESCE(original_json,result_json),result_json=?1,updated_at=?2,revision=revision+1,edited=1 WHERE id=?3 AND paper_id=?4 AND revision=?5",params![serde_json::to_string(&version.result).map_err(|_| failure())?,version.updated_at,id,paper_id,revision]).map_err(|_| failure())?;
        if changed != 1 {
            return Err(AppError::new(
                ErrorCode::DatabaseWriteFailed,
                "结果已在其他窗口修改，请刷新后再编辑。",
            ));
        }
        version.active = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM active_analyses WHERE paper_id=?1 AND version_id=?2)",
                params![paper_id, id],
                |row| row.get(0),
            )
            .map_err(|_| failure())?;
        if version.active {
            project(connection, &version)?;
        }
        version.revision = revision + 1;
        Ok(version.clone())
    })
}
pub fn backfill(connection: &Connection) -> AppResult<()> {
    let papers = SqlitePaperRepository::new(connection).list(&Default::default())?;
    for paper in papers {
        if let Some(mut result) = paper.analysis {
            result.tags = crate::services::keywords::normalize(&result.tags);
            let id = uuid::Uuid::new_v4().to_string();
            connection.execute("INSERT INTO analysis_versions(id,paper_id,provider,model,created_at,updated_at,result_json) VALUES(?1,?2,'legacy','历史结果（模型未记录）',?3,?4,?5)",params![id,paper.id,paper.created_at,paper.updated_at,serde_json::to_string(&result).map_err(|_| failure())?]).map_err(|_| failure())?;
            connection
                .execute(
                    "INSERT INTO active_analyses(paper_id,version_id) VALUES(?1,?2)",
                    params![paper.id, id],
                )
                .map_err(|_| failure())?;
            connection
                .execute(
                    "UPDATE papers SET tags_json=?1 WHERE id=?2",
                    params![serde_json::to_string(&result.tags).unwrap(), paper.id],
                )
                .map_err(|_| failure())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        crate::database::migrations::run_migrations(&mut c).unwrap();
        c.execute("INSERT INTO papers(id,file_path,file_name,title,created_at,updated_at) VALUES('p','source.pdf','source.pdf','Title','now','now')",[]).unwrap();
        c
    }
    fn result(summary: &str) -> PaperAnalysisResult {
        PaperAnalysisResult {
            schema_version: "2".into(),
            research_problem: "研究问题".into(),
            research_scenario: "研究场景".into(),
            core_method: "方法".into(),
            key_innovations: vec!["创新".into()],
            key_contributions: vec!["贡献".into()],
            main_results: "结果".into(),
            limitations: "局限".into(),
            tags: vec!["ISAC".into(), "通感融合".into()],
            ai_summary: summary.into(),
        }
    }
    #[test]
    fn multiple_models_edit_conflicts_projection_and_cascade() {
        let c = fixture();
        let a = append(&c, "p", "lmstudio", "model-a", &result("本地总结")).unwrap();
        let b = append(&c, "p", "codex", "model-b", &result("在线总结")).unwrap();
        assert_eq!(list(&c, "p").unwrap().len(), 2);
        assert_eq!(b.result.tags, ["通感一体化"]);
        let mut edited = a.result.clone();
        edited.ai_summary = "人工校订".into();
        edit(&c, "p", &a.id, 0, &edited).unwrap();
        assert_eq!(
            SqlitePaperRepository::new(&c)
                .find_by_id("p")
                .unwrap()
                .unwrap()
                .analysis
                .unwrap()
                .ai_summary,
            "在线总结"
        );
        assert!(edit(&c, "p", &a.id, 0, &result("过期编辑")).is_err());
        select(&c, "p", &a.id).unwrap();
        let paper = SqlitePaperRepository::new(&c)
            .find_by_id("p")
            .unwrap()
            .unwrap();
        assert_eq!(paper.analysis.unwrap().ai_summary, "人工校订");
        assert_eq!(paper.file_path, "source.pdf");
        let original: String = c
            .query_row(
                "SELECT original_json FROM analysis_versions WHERE id=?1",
                [&a.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(original.contains("本地总结"));
        assert!(select(&c, "other-paper", &a.id).is_err());
        assert!(append(&c, "missing", "codex", "m", &result("结果")).is_err());
        SqlitePaperRepository::new(&c).delete("p").unwrap();
        let count: i64 = c
            .query_row("SELECT count(*) FROM analysis_versions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
    #[test]
    fn migration_preserves_old_analysis_and_canonicalizes_search_projection() {
        let mut c = fixture();
        let mut paper = SqlitePaperRepository::new(&c)
            .find_by_id("p")
            .unwrap()
            .unwrap();
        paper.analysis = Some(result("迁移前总结"));
        paper.tags = vec!["ISAC".into()];
        SqlitePaperRepository::new(&c).update(&paper).unwrap();
        c.execute_batch(
            "DROP TABLE active_analyses; DROP TABLE analysis_versions; PRAGMA user_version=6",
        )
        .unwrap();
        crate::database::migrations::run_migrations(&mut c).unwrap();
        let versions = list(&c, "p").unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].provider, "legacy");
        assert_eq!(versions[0].result.ai_summary, "迁移前总结");
        assert_eq!(versions[0].result.tags, ["通感一体化"]);
    }
}
