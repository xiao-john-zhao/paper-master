import { invokeCommand } from "../adapters/tauri/invokeCommand";
import { open } from "@tauri-apps/plugin-dialog";

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
  async relocate() {
    const directory = await open({directory:true,multiple:false,title:"选择新缓存位置，或选择已经搬运的缓存文件夹"});
    if (!directory || Array.isArray(directory)) return undefined;
    return invokeCommand<LibraryCacheStatus>("relocate_library_cache", {directory});
  }
}
