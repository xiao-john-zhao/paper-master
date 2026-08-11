import { APP_CONFIG } from "../config/appConfig";
import type { AppSettings, SettingsStorage } from "./types";

export const DEFAULT_SETTINGS: AppSettings = {
  ai_provider: "lmstudio",
  base_url: APP_CONFIG.defaultBaseUrl,
  model: "deepseek-r1-distill-llama-8b",
  temperature: APP_CONFIG.defaultTemperature,
  request_timeout_ms: APP_CONFIG.defaultTimeoutMs,
  max_analysis_input_characters: APP_CONFIG.maxAnalysisInputCharacters,
  max_output_tokens: APP_CONFIG.maxOutputTokens,
  log_level: APP_CONFIG.logLevel,
  privacy_mode: "local_only",
};

export class InMemorySettingsStorage implements SettingsStorage {
  private settings = structuredClone(DEFAULT_SETTINGS);
  async get() { return structuredClone(this.settings); }
  async save(settings: AppSettings) { this.settings = structuredClone(settings); return this.get(); }
}
