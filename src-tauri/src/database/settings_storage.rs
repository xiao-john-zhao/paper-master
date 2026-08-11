use chrono::Utc;
use rusqlite::{params, Connection};

use crate::{
    domain::settings::AppSettings,
    error::{AppError, AppResult, ErrorCode},
};

pub trait SettingsStorage {
    fn get(&self) -> AppResult<AppSettings>;
    fn save(&self, settings: &AppSettings) -> AppResult<AppSettings>;
}

pub struct SqliteSettingsStorage<'connection> {
    connection: &'connection Connection,
}

impl<'connection> SqliteSettingsStorage<'connection> {
    pub fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }
}

impl SettingsStorage for SqliteSettingsStorage<'_> {
    fn get(&self) -> AppResult<AppSettings> {
        let result = self.connection.query_row(
            "SELECT value_json FROM settings WHERE key='app_settings'",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(value) => serde_json::from_str(&value).map_err(|error| {
                AppError::new(ErrorCode::InvalidSettings, "Stored settings are invalid.")
                    .detail(error.to_string())
            }),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(AppSettings::default()),
            Err(error) => Err(AppError::new(
                ErrorCode::DatabaseReadFailed,
                "Settings could not be loaded.",
            )
            .detail(error.to_string())),
        }
    }

    fn save(&self, settings: &AppSettings) -> AppResult<AppSettings> {
        validate(settings)?;
        let json = serde_json::to_string(settings).map_err(|error| {
            AppError::new(ErrorCode::InvalidSettings, "Settings could not be encoded.")
                .detail(error.to_string())
        })?;
        self.connection.execute("INSERT INTO settings(key,value_json,updated_at) VALUES('app_settings',?1,?2) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at", params![json, Utc::now().to_rfc3339()])
            .map_err(|error| AppError::new(ErrorCode::DatabaseWriteFailed, "Settings could not be saved.").detail(error.to_string()))?;
        Ok(settings.clone())
    }
}

fn validate(settings: &AppSettings) -> AppResult<()> {
    if !matches!(settings.ai_provider.as_str(), "lmstudio" | "mock") {
        return Err(invalid("Unknown AI provider."));
    }
    let parsed = url::Url::parse(&settings.base_url)
        .map_err(|_| invalid("Base URL must be a valid local HTTP URL."))?;
    let local_host = matches!(parsed.host_str(), Some("localhost") | Some("127.0.0.1"));
    if parsed.scheme() != "http"
        || !local_host
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(invalid(
            "Local Only mode permits only http://localhost or http://127.0.0.1.",
        ));
    }
    if !(0.0..=1.0).contains(&settings.temperature) {
        return Err(invalid("Temperature must be between 0 and 1."));
    }
    if !(30_000..=600_000).contains(&settings.request_timeout_ms) {
        return Err(invalid(
            "Request timeout must be between 30 and 600 seconds.",
        ));
    }
    if !(4_000..=100_000).contains(&settings.max_analysis_input_characters) {
        return Err(invalid(
            "Maximum analysis input must be between 4,000 and 100,000 characters.",
        ));
    }
    if settings.privacy_mode != "local_only" {
        return Err(invalid(
            "Only Local Only privacy mode is supported in v0.1.",
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidSettings, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_local_hosts() {
        let settings = AppSettings {
            base_url: "https://example.com/v1".into(),
            ..Default::default()
        };
        assert!(validate(&settings).is_err());
    }
    #[test]
    fn accepts_loopback_lm_studio() {
        assert!(validate(&AppSettings::default()).is_ok());
    }
}
