use crate::{
    database::{library_cache::CacheStatus, Database},
    error::{AppError, AppResult, ErrorCode},
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn get_library_cache_status(database: State<'_, Database>) -> AppResult<CacheStatus> {
    database.cache_status()
}

#[tauri::command]
pub fn sync_library_cache(database: State<'_, Database>) -> AppResult<CacheStatus> {
    database.sync_cache()
}

#[tauri::command]
pub fn recover_library_cache(database: State<'_, Database>) -> AppResult<CacheStatus> {
    database.recover_cache()
}

#[tauri::command]
pub fn open_library_cache_folder(app: AppHandle, database: State<'_, Database>) -> AppResult<()> {
    let status = database.cache_status()?;
    std::fs::create_dir_all(&status.directory)
        .map_err(|_| AppError::new(ErrorCode::FileNotFound, "缓存文件夹无法创建。"))?;
    app.opener()
        .open_path(status.directory, None::<&str>)
        .map_err(|_| AppError::new(ErrorCode::FileNotFound, "缓存文件夹无法打开。"))
}
