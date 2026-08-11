import { invokeCommand } from "../adapters/tauri/invokeCommand";
import type { AppSettings, SettingsStorage } from "./types";

export class TauriSettingsStorage implements SettingsStorage {
  get() { return invokeCommand<AppSettings>("get_settings"); }
  save(settings: AppSettings) { return invokeCommand<AppSettings>("save_settings", { settings }); }
}

