export type AppErrorCode =
  | "PDF_READ_FAILED"
  | "PDF_TEXT_EMPTY"
  | "DATABASE_WRITE_FAILED"
  | "DATABASE_READ_FAILED"
  | "LMSTUDIO_CONNECTION_FAILED"
  | "LMSTUDIO_MODEL_NOT_FOUND"
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
}

export class AppError extends Error {
  readonly code: AppErrorCode;
  readonly detail?: string;
  readonly requestId?: string;

  constructor(payload: AppErrorPayload) {
    super(payload.message);
    this.name = "AppError";
    this.code = payload.code;
    this.detail = payload.detail;
    this.requestId = payload.request_id;
  }
}

export function normalizeAppError(error: unknown): AppError {
  if (error instanceof AppError) return error;
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    return new AppError(error as AppErrorPayload);
  }
  return new AppError({ code: "UNKNOWN", message: error instanceof Error ? error.message : String(error) });
}
