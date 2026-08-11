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

const MIGRATIONS: &[(i64, &str)] = &[(1, MIGRATION_1)];

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
        assert_eq!(run_migrations(&mut connection).expect("first migration"), 1);
        assert_eq!(
            run_migrations(&mut connection).expect("second migration"),
            1
        );
        let paper_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='papers'",
                [],
                |row| row.get(0),
            )
            .expect("papers table");
        assert_eq!(paper_table, "papers");
    }
}
