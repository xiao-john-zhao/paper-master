export type AIProviderId = "lmstudio" | "mock" | "codex" | "openai";

export interface AppSettings {
  ai_provider: AIProviderId;
  auto_analyze_on_import: boolean;
  reasoning_effort: "none" | "low" | "medium" | "high" | "xhigh";
  model_context_length: number;
  base_url: string;
  api_base_url: string;
  api_key: string;
  api_model: string;
  api_response_format: "json_schema" | "json_object";
  api_reasoning_supported: boolean;
  adaptive_analysis: boolean;
  model: string;
  temperature: number;
  request_timeout_ms: number;
  max_analysis_input_characters: number;
  max_output_tokens: number;
  log_level: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";
  privacy_mode: "local_only" | "codex_online";
}

export interface SettingsStorage {
  get(): Promise<AppSettings>;
  save(settings: AppSettings): Promise<AppSettings>;
}
