import { APP_CONFIG } from "../config/appConfig";
import type { AppSettings, SettingsStorage } from "./types";

export const DEFAULT_SETTINGS: AppSettings = {
  ai_provider: "lmstudio",
  auto_analyze_on_import: true,
  reasoning_effort: "none",
  model_context_length: 24_576,
  base_url: APP_CONFIG.defaultBaseUrl,
  api_base_url: "",
  api_key: "",
  api_model: "",
  api_response_format: "json_object",
  api_reasoning_supported: false,
  adaptive_analysis: true,
  model: "",
  temperature: APP_CONFIG.defaultTemperature,
  request_timeout_ms: APP_CONFIG.defaultTimeoutMs,
  max_analysis_input_characters: APP_CONFIG.maxAnalysisInputCharacters,
  max_output_tokens: APP_CONFIG.maxOutputTokens,
  log_level: APP_CONFIG.logLevel,
  privacy_mode: "local_only",
};

export class InMemorySettingsStorage implements SettingsStorage {
  private settings: AppSettings;
  constructor(initial: AppSettings = DEFAULT_SETTINGS) { this.settings = structuredClone(initial); }
  async get() { return structuredClone(this.settings); }
  async save(settings: AppSettings) { this.settings = structuredClone(settings); return this.get(); }
}
