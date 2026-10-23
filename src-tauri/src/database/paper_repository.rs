use rusqlite::{params, types::Type, Connection, Row};
use std::collections::HashSet;

use crate::{
    domain::paper::{Paper, PaperAnalysisResult, PaperQuery},
    error::{AppError, AppResult, ErrorCode},
};

pub trait PaperRepository {
    fn create(&self, paper: &Paper, extracted_text: &str) -> AppResult<Paper>;
    fn update(&self, paper: &Paper) -> AppResult<Paper>;
    fn delete(&self, id: &str) -> AppResult<()>;
    fn find_by_id(&self, id: &str) -> AppResult<Option<Paper>>;
    fn list(&self, query: &PaperQuery) -> AppResult<Vec<Paper>>;
    fn search(&self, query: &PaperQuery) -> AppResult<Vec<Paper>>;
    fn exists_by_path(&self, path: &str) -> AppResult<bool>;
}

pub struct SqlitePaperRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> SqlitePaperRepository<'connection> {
    pub fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub fn extracted_text(&self, id: &str) -> AppResult<String> {
        self.connection
            .query_row(
                "SELECT extracted_text FROM papers WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .map_err(|error| read_error("Could not load extracted paper text", error))
    }

    pub fn counts(&self) -> AppResult<(usize, usize)> {
        self.connection
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(ai_analyzed), 0) FROM papers",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? as usize,
                        row.get::<_, i64>(1)? as usize,
                    ))
                },
            )
            .map_err(|error| read_error("Could not count papers", error))
    }

    fn all(&self) -> AppResult<Vec<Paper>> {
        let mut statement = self
            .connection
            .prepare(PAPER_SELECT)
            .map_err(|error| read_error("Could not prepare paper query", error))?;
        let papers = statement
            .query_map([], map_paper)
            .map_err(|error| read_error("Could not read papers", error))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| read_error("A stored paper record is invalid", error))?;
        Ok(papers)
    }

    fn fts_ids(&self, query: &str) -> Result<HashSet<String>, rusqlite::Error> {
        let expression = fts_expression(query);
        if expression.is_empty() {
            return Ok(HashSet::new());
        }
        let mut statement = self.connection.prepare(
            "SELECT paper_id FROM papers_fts WHERE papers_fts MATCH ?1 ORDER BY rank LIMIT 500",
        )?;
        let ids = statement
            .query_map([expression], |row| row.get::<_, String>(0))?
            .collect();
        ids
    }
}

impl PaperRepository for SqlitePaperRepository<'_> {
    fn create(&self, paper: &Paper, extracted_text: &str) -> AppResult<Paper> {
        let analysis = paper.analysis.as_ref();
        self.connection.execute(
            "INSERT INTO papers (id,file_path,file_name,title,authors_json,year,abstract_text,extracted_text,ai_summary,research_problem,core_method,key_contributions_json,main_results,limitations,tags_json,ai_analyzed,analysis_status,analysis_error,pdf_diagnostics_json,created_at,updated_at,analysis_schema_version,research_scenario,key_innovations_json,publication_venue) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
            params![
                paper.id, paper.file_path, paper.file_name, paper.title, json(&paper.authors)?, paper.year, paper.abstract_text,
                extracted_text, analysis.map(|value| value.ai_summary.as_str()), analysis.map(|value| value.research_problem.as_str()),
                analysis.map(|value| value.core_method.as_str()), json(&analysis.map(|value| &value.key_contributions).cloned().unwrap_or_default())?,
                analysis.map(|value| value.main_results.as_str()), analysis.map(|value| value.limitations.as_str()), json(&paper.tags)?,
                i64::from(paper.analysis.is_some()), paper.analysis_status, paper.analysis_error, optional_json(paper.pdf_diagnostics.as_ref())?, paper.created_at, paper.updated_at,
                analysis.map(|value| value.schema_version.as_str()).unwrap_or("2"), analysis.map(|value| value.research_scenario.as_str()),
                json(&analysis.map(|value| &value.key_innovations).cloned().unwrap_or_default())?,
                paper.publication_venue,
            ],
        ).map_err(|error| {
            if matches!(&error, rusqlite::Error::SqliteFailure(inner, _) if inner.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE) {
                AppError::new(ErrorCode::DuplicatePaper, "This PDF is already in the library.").detail(error.to_string())
            } else { write_error("Could not insert paper", error) }
        })?;
        Ok(paper.clone())
    }

    fn update(&self, paper: &Paper) -> AppResult<Paper> {
        let analysis = paper.analysis.as_ref();
        let changed = self.connection.execute(
            "UPDATE papers SET file_path=?2,file_name=?3,title=?4,authors_json=?5,year=?6,abstract_text=?7,ai_summary=?8,research_problem=?9,core_method=?10,key_contributions_json=?11,main_results=?12,limitations=?13,tags_json=?14,ai_analyzed=?15,analysis_status=?16,analysis_error=?17,pdf_diagnostics_json=?18,updated_at=?19,analysis_schema_version=?20,research_scenario=?21,key_innovations_json=?22,publication_venue=?23 WHERE id=?1",
            params![paper.id,paper.file_path,paper.file_name,paper.title,json(&paper.authors)?,paper.year,paper.abstract_text,
                analysis.map(|value| value.ai_summary.as_str()),analysis.map(|value| value.research_problem.as_str()),analysis.map(|value| value.core_method.as_str()),
                json(&analysis.map(|value| &value.key_contributions).cloned().unwrap_or_default())?,analysis.map(|value| value.main_results.as_str()),analysis.map(|value| value.limitations.as_str()),
                json(&paper.tags)?,i64::from(paper.analysis.is_some()),paper.analysis_status,paper.analysis_error,optional_json(paper.pdf_diagnostics.as_ref())?,paper.updated_at,
                analysis.map(|value| value.schema_version.as_str()).unwrap_or("2"),analysis.map(|value| value.research_scenario.as_str()),
                json(&analysis.map(|value| &value.key_innovations).cloned().unwrap_or_default())?,paper.publication_venue],
        ).map_err(|error| write_error("Could not update paper", error))?;
        if changed == 0 {
            return Err(AppError::new(
                ErrorCode::DatabaseWriteFailed,
                "The paper no longer exists in the library.",
            ));
        }
        Ok(paper.clone())
    }

    fn delete(&self, id: &str) -> AppResult<()> {
        self.connection
            .execute("DELETE FROM papers WHERE id=?1", [id])
            .map_err(|error| write_error("Could not delete paper", error))?;
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> AppResult<Option<Paper>> {
        let mut statement = self
            .connection
            .prepare(&format!("{PAPER_SELECT} WHERE id=?1"))
            .map_err(|error| read_error("Could not prepare paper lookup", error))?;
        match statement.query_row([id], map_paper) {
            Ok(paper) => Ok(Some(paper)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(read_error("Could not read paper", error)),
        }
    }

    fn list(&self, query: &PaperQuery) -> AppResult<Vec<Paper>> {
        Ok(self
            .all()?
            .into_iter()
            .filter(|paper| matches_query(paper, query))
            .collect())
    }

    fn search(&self, query: &PaperQuery) -> AppResult<Vec<Paper>> {
        let Some(text) = query
            .text
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        else {
            return self.list(query);
        };
        match self.fts_ids(text) {
            Ok(ids) => {
                let non_text_query = PaperQuery {
                    text: None,
                    tag: query.tag.clone(),
                    filter: query.filter.clone(),
                };
                Ok(self
                    .all()?
                    .into_iter()
                    .filter(|paper| {
                        (ids.contains(&paper.id)
                            || paper.tags.iter().any(|tag| {
                                crate::services::keywords::canonical(tag).eq_ignore_ascii_case(
                                    &crate::services::keywords::canonical(text),
                                )
                            }))
                            && matches_query(paper, &non_text_query)
                    })
                    .collect())
            }
            Err(_) => self.list(query),
        }
    }

    fn exists_by_path(&self, path: &str) -> AppResult<bool> {
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM papers WHERE file_path=?1)",
                [path],
                |row| row.get(0),
            )
            .map_err(|error| read_error("Could not check duplicate path", error))
    }
}

fn fts_expression(query: &str) -> String {
    query
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|character| character.is_alphanumeric() || matches!(character, '-' | '_'))
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .take(12)
        .map(|token| format!("\"{}\"*", token.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

const PAPER_SELECT: &str = "SELECT id,file_path,file_name,title,authors_json,year,abstract_text,ai_summary,research_problem,core_method,key_contributions_json,main_results,limitations,tags_json,ai_analyzed,analysis_status,analysis_error,pdf_diagnostics_json,created_at,updated_at,analysis_schema_version,research_scenario,key_innovations_json,publication_venue,(SELECT v.provider FROM analysis_versions v JOIN active_analyses a ON a.version_id=v.id WHERE a.paper_id=papers.id) FROM papers";

fn map_paper(row: &Row<'_>) -> rusqlite::Result<Paper> {
    let authors: Vec<String> = parse_json(row.get::<_, String>(4)?, 4)?;
    let key_contributions: Vec<String> = parse_json(row.get::<_, String>(10)?, 10)?;
    let tags: Vec<String> = parse_json(row.get::<_, String>(13)?, 13)?;
    let analyzed = row.get::<_, i64>(14)? == 1;
    let ai_summary: Option<String> = row.get(7)?;
    let research_problem: Option<String> = row.get(8)?;
    let core_method: Option<String> = row.get(9)?;
    let main_results: Option<String> = row.get(11)?;
    let limitations: Option<String> = row.get(12)?;
    let diagnostics_json: Option<String> = row.get(17)?;
    let schema_version: String = row.get(20)?;
    let research_scenario: Option<String> = row.get(21)?;
    let key_innovations: Vec<String> = parse_json(row.get::<_, String>(22)?, 22)?;
    let analysis = analyzed.then(|| PaperAnalysisResult {
        schema_version,
        research_problem: research_problem.unwrap_or_default(),
        research_scenario: research_scenario.unwrap_or_default(),
        core_method: core_method.unwrap_or_default(),
        key_innovations,
        key_contributions,
        main_results: main_results.unwrap_or_default(),
        limitations: limitations.unwrap_or_default(),
        tags: tags.clone(),
        ai_summary: ai_summary.unwrap_or_default(),
    });
    Ok(Paper {
        id: row.get(0)?,
        file_path: row.get(1)?,
        file_name: row.get(2)?,
        title: row.get(3)?,
        publication_venue: row.get(23)?,
        authors,
        year: row.get(5)?,
        abstract_text: row.get(6)?,
        analysis,
        analysis_provider: row.get(24)?,
        tags,
        analysis_status: row.get(15)?,
        analysis_error: row.get(16)?,
        pdf_diagnostics: match diagnostics_json {
            Some(value) => Some(parse_json(value, 17)?),
            None => None,
        },
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn matches_query(paper: &Paper, query: &PaperQuery) -> bool {
    if query.filter.as_deref() == Some("analyzed") && paper.analysis_status != "analyzed" {
        return false;
    }
    if query.filter.as_deref() == Some("recent") {
        let created = chrono::DateTime::parse_from_rfc3339(&paper.created_at)
            .map(|value| value.with_timezone(&chrono::Utc));
        if created
            .map(|value| chrono::Utc::now().signed_duration_since(value).num_days() > 7)
            .unwrap_or(true)
        {
            return false;
        }
    }
    if let Some(tag) = query.tag.as_ref().filter(|value| !value.trim().is_empty()) {
        if !paper.tags.iter().any(|value| {
            crate::services::keywords::canonical(value)
                .eq_ignore_ascii_case(&crate::services::keywords::canonical(tag))
        }) {
            return false;
        }
    }
    let Some(text) = query
        .text
        .as_ref()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
    else {
        return true;
    };
    if paper.tags.iter().any(|tag| {
        crate::services::keywords::canonical(tag)
            .eq_ignore_ascii_case(&crate::services::keywords::canonical(&text))
    }) {
        return true;
    }
    let analysis = paper.analysis.as_ref();
    [
        paper.title.as_str(),
        paper.publication_venue.as_deref().unwrap_or_default(),
        &paper.authors.join(" "),
        paper.abstract_value().unwrap_or_default(),
        analysis
            .map(|value| value.ai_summary.as_str())
            .unwrap_or_default(),
        analysis
            .map(|value| value.research_problem.as_str())
            .unwrap_or_default(),
        analysis
            .map(|value| value.research_scenario.as_str())
            .unwrap_or_default(),
        analysis
            .map(|value| value.core_method.as_str())
            .unwrap_or_default(),
        &analysis
            .map(|value| value.key_innovations.join(" "))
            .unwrap_or_default(),
        &analysis
            .map(|value| value.key_contributions.join(" "))
            .unwrap_or_default(),
        &paper.tags.join(" "),
    ]
    .iter()
    .any(|value| value.to_lowercase().contains(&text))
}

fn json<T: serde::Serialize>(value: &T) -> AppResult<String> {
    serde_json::to_string(value).map_err(|error| {
        AppError::new(
            ErrorCode::DatabaseWriteFailed,
            "Paper data could not be encoded.",
        )
        .detail(error.to_string())
    })
}
fn optional_json<T: serde::Serialize>(value: Option<&T>) -> AppResult<Option<String>> {
    value.map(json).transpose()
}
fn parse_json<T: serde::de::DeserializeOwned>(value: String, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}
fn read_error(context: &str, error: rusqlite::Error) -> AppError {
    AppError::new(
        ErrorCode::DatabaseReadFailed,
        "The local library could not be read.",
    )
    .detail(format!("{context}: {error}"))
}
fn write_error(context: &str, error: rusqlite::Error) -> AppError {
    AppError::new(
        ErrorCode::DatabaseWriteFailed,
        "The local library could not be updated.",
    )
    .detail(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    fn paper() -> Paper {
        Paper {
            id: "paper-1".into(),
            file_path: "C:\\papers\\one.pdf".into(),
            file_name: "one.pdf".into(),
            title: "Local Models for Science".into(),
            publication_venue: Some("Journal of Local AI".into()),
            authors: vec!["Ada Lovelace".into()],
            year: Some(2025),
            abstract_text: Some("Local scientific analysis".into()),
            analysis: None,
            analysis_provider: None,
            tags: vec!["local LLM".into()],
            analysis_status: "not_analyzed".into(),
            analysis_error: None,
            pdf_diagnostics: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn repository_crud_and_search() {
        let database = Database::in_memory().expect("database");
        let connection = database.connection().expect("connection");
        let repository = SqlitePaperRepository::new(&connection);
        let value = paper();
        repository.create(&value, "extracted").expect("create");
        assert!(repository.exists_by_path(&value.file_path).expect("exists"));
        assert_eq!(
            repository
                .search(&PaperQuery {
                    text: Some("lovelace".into()),
                    ..Default::default()
                })
                .expect("search")
                .len(),
            1
        );
        assert!(repository
            .search(&PaperQuery {
                text: Some("\" OR *".into()),
                ..Default::default()
            })
            .expect("safe query")
            .is_empty());
        assert_eq!(
            repository.extracted_text(&value.id).expect("text"),
            "extracted"
        );
        repository.delete(&value.id).expect("delete");
        assert!(repository.find_by_id(&value.id).expect("find").is_none());
    }

    #[test]
    fn persists_and_searches_v2_summary_modules() {
        let database = Database::in_memory().expect("database");
        let connection = database.connection().expect("connection");
        let repository = SqlitePaperRepository::new(&connection);
        let mut value = paper();
        value.analysis = Some(PaperAnalysisResult {
            schema_version: "2".into(),
            research_problem: "统一文献归纳问题".into(),
            research_scenario: "本地科研文献管理场景".into(),
            core_method: "结构化提示与响应校验".into(),
            key_innovations: vec!["五模块强制契约".into()],
            key_contributions: vec!["提升跨论文总结一致性".into()],
            main_results: "生成稳定的模块化总结".into(),
            limitations: "依赖输入论文中的明确证据".into(),
            tags: vec!["文献总结".into(), "结构化输出".into(), "本地大模型".into()],
            ai_summary: "以统一契约生成模块化论文总结。".into(),
        });
        value.tags = value.analysis.as_ref().expect("analysis").tags.clone();
        value.analysis_status = "analyzed".into();
        repository.create(&value, "extracted").expect("create");

        let stored = repository
            .find_by_id(&value.id)
            .expect("find")
            .expect("paper")
            .analysis
            .expect("stored analysis");
        assert_eq!(stored.schema_version, "2");
        assert_eq!(stored.research_scenario, "本地科研文献管理场景");
        assert_eq!(stored.key_innovations, vec!["五模块强制契约"]);
        assert_eq!(
            repository
                .search(&PaperQuery {
                    text: Some("五模块".into()),
                    ..Default::default()
                })
                .expect("search")
                .len(),
            1
        );
    }

    #[test]
    fn fts_query_is_bounded_and_sanitized() {
        assert_eq!(fts_expression("local LLM"), "\"local\"* AND \"LLM\"*");
        assert_eq!(fts_expression("\" OR *"), "\"OR\"*");
    }
}
