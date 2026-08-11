type Level = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";

interface LogContext {
  errorCode?: string;
  requestId?: string;
  durationMs?: number;
  detail?: string;
}

function write(level: Level, operation: string, message: string, context: LogContext = {}) {
  const entry = { timestamp: new Date().toISOString(), level, module: "ui", operation, message, ...context };
  if (import.meta.env.DEV) {
    const method = level === "ERROR" ? console.error : level === "WARN" ? console.warn : console.debug;
    method("[paper-master]", entry);
  }
  if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
    void invoke("write_ui_log", {
      event: { operation, message, request_id: context.requestId },
    }).catch(() => undefined);
  }
}

export const uiLogger = {
  trace: (operation: string, message: string, context?: LogContext) => write("TRACE", operation, message, context),
  debug: (operation: string, message: string, context?: LogContext) => write("DEBUG", operation, message, context),
  info: (operation: string, message: string, context?: LogContext) => write("INFO", operation, message, context),
  warn: (operation: string, message: string, context?: LogContext) => write("WARN", operation, message, context),
  error: (operation: string, message: string, context?: LogContext) => write("ERROR", operation, message, context),
};
import { invoke } from "@tauri-apps/api/core";

