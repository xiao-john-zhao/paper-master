import { invoke } from "@tauri-apps/api/core";
import { normalizeAppError } from "../../types/errors";

export async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeAppError(error);
  }
}

export function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

