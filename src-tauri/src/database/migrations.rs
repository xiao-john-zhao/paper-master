use rusqlite::Connection;

use crate::error::{AppError, AppResult, ErrorCode};

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS papers (
    id TEXT PRIMARY KEY NOT NULL,
    file_path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    title TEXT NOT NULL,
    authors_json TEXT NOT NULL DEFAULT '[]',
    year INTEGER,
    abstract_text TEXT,
    extracted_text TEXT NOT NULL DEFAULT '',
    ai_summary TEXT,
    research_problem TEXT,
    core_method TEXT,
    key_contributions_json TEXT NOT NULL DEFAULT '[]',
    main_results TEXT,
    limitations TEXT,
    tags_json TEXT NOT NULL DEFAULT '[]',
    ai_analyzed INTEGER NOT NULL DEFAULT 0 CHECK (ai_analyzed IN (0, 1)),
    analysis_status TEXT NOT NULL DEFAULT 'not_analyzed',
    analysis_error TEXT,
    pdf_diagnostics_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_papers_created_at ON papers(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_papers_ai_analyzed ON papers(ai_analyzed, updated_at DESC);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS operation_status (
    operation_id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL
);
"#;

const MIGRATION_2: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
    paper_id UNINDEXED,
    title,
    authors,
    abstract_text,
    ai_summary,
    research_problem,
    core_method,
    key_contributions,
    tags,
    tokenize = 'unicode61'
);

INSERT INTO papers_fts(paper_id,title,authors,abstract_text,ai_summary,research_problem,core_method,key_contributions,tags)
SELECT id,title,authors_json,COALESCE(abstract_text,''),COALESCE(ai_summary,''),COALESCE(research_problem,''),COALESCE(core_method,''),key_contributions_json,tags_json
FROM papers
WHERE id NOT IN (SELECT paper_id FROM papers_fts);

CREATE TRIGGER IF NOT EXISTS papers_fts_insert AFTER INSERT ON papers BEGIN
    INSERT INTO papers_fts(paper_id,title,authors,abstract_text,ai_summary,research_problem,core_method,key_contributions,tags)
    VALUES(new.id,new.title,new.authors_json,COALESCE(new.abstract_text,''),COALESCE(new.ai_summary,''),COALESCE(new.research_problem,''),COALESCE(new.core_method,''),new.key_contributions_json,new.tags_json);
END;

CREATE TRIGGER IF NOT EXISTS papers_fts_update AFTER UPDATE ON papers BEGIN
    DELETE FROM papers_fts WHERE paper_id=old.id;
    INSERT INTO papers_fts(paper_id,title,authors,abstract_text,ai_summary,research_problem,core_method,key_contributions,tags)
    VALUES(new.id,new.title,new.authors_json,COALESCE(new.abstract_text,''),COALESCE(new.ai_summary,''),COALESCE(new.research_problem,''),COALESCE(new.core_method,''),new.key_contributions_json,new.tags_json);
END;

CREATE TRIGGER IF NOT EXISTS papers_fts_delete AFTER DELETE ON papers BEGIN
    DELETE FROM papers_fts WHERE paper_id=old.id;
END;
"#;

const MIGRATION_3: &str = r#"
UPDATE OR IGNORE papers
SET file_path = CASE
    WHEN substr(file_path, 1, 8) = '\\?\UNC\' THEN '\\' || substr(file_path, 9)
    WHEN substr(file_path, 1, 4) = '\\?\' THEN substr(file_path, 5)
    ELSE file_path
END
WHERE substr(file_path, 1, 4) = '\\?\';
"#;

const MIGRATIONS: &[(i64, &str)] = &[(1, MIGRATION_1), (2, MIGRATION_2), (3, MIGRATION_3)];

pub fn run_migrations(connection: &mut Connection) -> AppResult<i64> {
    connection
        .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|error| migration_error("Database pragmas failed", error))?;
    let current: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| migration_error("Could not read migration version", error))?;

    let mut applied = current;
    for (version, sql) in MIGRATIONS.iter().filter(|(version, _)| *version > current) {
        let transaction = connection
            .transaction()
            .map_err(|error| migration_error("Could not start migration", error))?;
        transaction
            .execute_batch(sql)
            .map_err(|error| migration_error(&format!("Migration {version} failed"), error))?;
        transaction
            .pragma_update(None, "user_version", version)
            .map_err(|error| migration_error("Could not record migration version", error))?;
        transaction
            .commit()
            .map_err(|error| migration_error("Could not commit migration", error))?;
        applied = *version;
    }
    Ok(applied)
}

fn migration_error(message: &str, error: rusqlite::Error) -> AppError {
    AppError::new(
        ErrorCode::MigrationFailed,
        "The local library database could not be upgraded.",
    )
    .detail(format!("{message}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let mut connection = Connection::open_in_memory().expect("in-memory database");
        assert_eq!(run_migrations(&mut connection).expect("first migration"), 3);
        assert_eq!(
            run_migrations(&mut connection).expect("second migration"),
            3
        );
        let paper_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='papers'",
                [],
                |row| row.get(0),
            )
            .expect("papers table");
        assert_eq!(paper_table, "papers");
        let fts_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE name='papers_fts'",
                [],
                |row| row.get(0),
            )
            .expect("fts table");
        assert_eq!(fts_table, "papers_fts");
    }

    #[test]
    fn migration_normalizes_windows_extended_paths_without_losing_unicode() {
        let mut connection = Connection::open_in_memory().expect("in-memory database");
        connection.execute_batch(MIGRATION_1).expect("base schema");
        connection
            .execute(
                "INSERT INTO papers(id,file_path,file_name,title,created_at,updated_at) VALUES('p1',?1,'测试.pdf','中文论文','now','now')",
                [r"\\?\E:\中文目录\测试.pdf"],
            )
            .expect("fixture");
        connection
            .pragma_update(None, "user_version", 2)
            .expect("version");
        assert_eq!(run_migrations(&mut connection).expect("migration"), 3);
        let path: String = connection
            .query_row("SELECT file_path FROM papers WHERE id='p1'", [], |row| {
                row.get(0)
            })
            .expect("path");
        assert_eq!(path, r"E:\中文目录\测试.pdf");
    }
}
