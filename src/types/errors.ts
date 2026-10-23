export type AppErrorCode =
  | "CODEX_UNAVAILABLE"
  | "RESEARCH_FAILED"
  | "PDF_READ_FAILED"
  | "PDF_TEXT_EMPTY"
  | "DATABASE_WRITE_FAILED"
  | "DATABASE_READ_FAILED"
  | "LMSTUDIO_CONNECTION_FAILED"
  | "LMSTUDIO_MODEL_NOT_FOUND"
  | "LMSTUDIO_INVALID_RESPONSE"
  | "AI_FINAL_CONTENT_EMPTY"
  | "AI_OUTPUT_TOKEN_LIMIT"
  | "AI_SCHEMA_VALIDATION_FAILED"
  | "MODEL_STRUCTURED_OUTPUT_INCOMPATIBLE"
  | "AI_TIMEOUT"
  | "AI_CANCELLED"
  | "AI_INVALID_JSON"
  | "FILE_NOT_FOUND"
  | "DUPLICATE_PAPER"
  | "PAPER_NOT_ANALYZED"
  | "INVALID_SETTINGS"
  | "UI_RENDER_FAILED"
  | "UNKNOWN";

export interface AppErrorPayload {
  code: AppErrorCode;
  message: string;
  detail?: string;
  request_id?: string;
  stage?: string;
}

export class AppError extends Error {
  readonly code: AppErrorCode;
  readonly detail?: string;
  readonly requestId?: string;
  readonly stage?: string;

  constructor(payload: AppErrorPayload) {
    super(payload.message);
    this.name = "AppError";
    this.code = payload.code;
    this.detail = payload.detail;
    this.requestId = payload.request_id;
    this.stage = payload.stage;
  }
}

export function normalizeAppError(error: unknown): AppError {
  if (error instanceof AppError) return error;
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    return new AppError(error as AppErrorPayload);
  }
  return new AppError({ code: "UNKNOWN", message: error instanceof Error ? error.message : String(error) });
}
