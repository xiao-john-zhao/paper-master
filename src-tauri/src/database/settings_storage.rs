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
    if !matches!(settings.ai_provider.as_str(), "lmstudio" | "mock" | "codex") {
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
    if !matches!(
        settings.reasoning_effort.as_str(),
        "none" | "off" | "low" | "medium" | "high" | "xhigh"
    ) {
        return Err(invalid("Unsupported reasoning effort."));
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
    if !(512..=32_768).contains(&settings.max_output_tokens) {
        return Err(invalid(
            "Maximum AI output must be between 512 and 32,768 tokens.",
        ));
    }
    if !(4096..=131_072).contains(&settings.model_context_length)
        || settings.max_output_tokens + 2048 >= settings.model_context_length
    {
        return Err(invalid(
            "模型上下文需在 4096–131072 token 范围，且应为输入预留至少 2048 token。",
        ));
    }
    if !matches!(
        settings.privacy_mode.as_str(),
        "local_only" | "codex_online"
    ) || (settings.ai_provider == "codex" && settings.privacy_mode != "codex_online")
    {
        return Err(invalid(
            "Codex 模式需要明确启用联网授权；本地模型仍仅允许回环地址。",
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
        let settings = AppSettings::default();
        assert_eq!(settings.request_timeout_ms, 600_000);
        assert!(validate(&settings).is_ok());
    }
    #[test]
    fn codex_requires_explicit_online_mode_and_keeps_lmstudio_local() {
        let mut settings = AppSettings {
            ai_provider: "codex".into(),
            ..Default::default()
        };
        assert!(validate(&settings).is_err());
        settings.privacy_mode = "codex_online".into();
        assert!(validate(&settings).is_ok());
        settings.base_url = "https://example.com/v1".into();
        assert!(validate(&settings).is_err());
    }

    #[test]
    fn existing_settings_enable_auto_analysis_without_resetting_model() {
        let mut json = serde_json::to_value(AppSettings::default()).unwrap();
        json.as_object_mut()
            .unwrap()
            .remove("auto_analyze_on_import");
        json["model"] = "existing-model".into();
        let settings: AppSettings = serde_json::from_value(json).unwrap();
        assert!(settings.auto_analyze_on_import);
        assert_eq!(settings.model, "existing-model");
    }
}
