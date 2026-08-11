export type AIProviderId = "lmstudio" | "mock";

export interface AppSettings {
  ai_provider: AIProviderId;
  base_url: string;
  model: string;
  temperature: number;
  request_timeout_ms: number;
  max_analysis_input_characters: number;
  max_output_tokens: number;
  log_level: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";
  privacy_mode: "local_only";
}

export interface SettingsStorage {
  get(): Promise<AppSettings>;
  save(settings: AppSettings): Promise<AppSettings>;
}
