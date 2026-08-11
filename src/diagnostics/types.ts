import type { ConnectionState } from "../domain/ai/types";

export interface DiagnosticSnapshot {
  application_version: string;
  environment: string;
  database_path: string;
  database_status: string;
  migration_version: number;
  lm_studio_base_url: string;
  lm_studio_status: ConnectionState;
  current_model: string;
  paper_count: number;
  analyzed_paper_count: number;
  last_ai_request_status: string;
  last_import_status: string;
  recent_errors: Array<{ timestamp: string; module: string; operation: string; message: string; error_code?: string; request_id?: string }>;
  log_file_location: string;
}

