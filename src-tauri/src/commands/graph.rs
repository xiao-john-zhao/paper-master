use crate::error::{AppError, AppResult, ErrorCode};
use tauri_plugin_dialog::DialogExt;

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::Internal, message)
}
#[tauri::command]
pub async fn save_graph_image(
    app: tauri::AppHandle,
    format: String,
    data: Vec<u8>,
) -> AppResult<Option<String>> {
    validate_image(&format, &data)?;
    tauri::async_runtime::spawn_blocking(move || {
        let name = format!("paper-master-relations.{format}");
        let Some(file) = app
            .dialog()
            .file()
            .set_file_name(&name)
            .add_filter("关系图片", &[format.as_str()])
            .blocking_save_file()
        else {
            return Ok(None);
        };
        let mut path = file
            .into_path()
            .map_err(|_| invalid("只能保存到本地文件路径。"))?;
        if path.extension().is_none() {
            path.set_extension(&format);
        }
        if !path
            .extension()
            .is_some_and(|value| value.eq_ignore_ascii_case(&format))
        {
            return Err(invalid("文件扩展名需与导出格式一致（PNG 或 SVG）。"));
        }
        std::fs::write(&path, data).map_err(|_| invalid("无法保存关系图片，请检查目录权限。"))?;
        Ok(Some(path.to_string_lossy().to_string()))
    })
    .await
    .map_err(|_| invalid("图片保存任务异常退出。"))?
}
fn validate_image(format: &str, data: &[u8]) -> AppResult<()> {
    if data.len() > 12 * 1024 * 1024 || data.is_empty() {
        return Err(invalid("图片为空或超过 12 MB 导出限制。"));
    }
    match format {
        "png" if data.starts_with(b"\x89PNG\r\n\x1a\n") => Ok(()),
        "svg"
            if std::str::from_utf8(data)
                .is_ok_and(|text| text.starts_with("<svg ") && text.ends_with("</svg>")) =>
        {
            Ok(())
        }
        _ => Err(invalid("导出数据不是有效的 PNG 或 SVG 图片。")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_image_payloads_and_unsupported_formats() {
        assert!(validate_image("pdf", b"%PDF-").is_err());
        assert!(validate_image("png", b"<html>login</html>").is_err());
        assert!(validate_image("svg", b"<svg xmlns='http://www.w3.org/2000/svg'></svg>").is_ok());
        assert!(validate_image("png", b"\x89PNG\r\n\x1a\n").is_ok());
    }
}
