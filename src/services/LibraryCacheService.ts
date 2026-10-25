import { invokeCommand } from "../adapters/tauri/invokeCommand";

export interface LibraryCacheStatus {
  directory: string;
  schema_version: number;
  pending: number;
  restored: number;
  unreadable: number;
  last_error?: string | null;
}

export class LibraryCacheService {
  status() { return invokeCommand<LibraryCacheStatus>("get_library_cache_status"); }
  sync() { return invokeCommand<LibraryCacheStatus>("sync_library_cache"); }
  recover() { return invokeCommand<LibraryCacheStatus>("recover_library_cache"); }
  openFolder() { return invokeCommand<void>("open_library_cache_folder"); }
}
