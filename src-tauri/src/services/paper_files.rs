use crate::{
    error::{AppError, AppResult, ErrorCode},
    services::paper_import::normalize_storage_path,
};
use std::path::{Path, PathBuf};

pub fn existing_pdf(path: &str) -> AppResult<PathBuf> {
    let normalized = normalize_storage_path(Path::new(path));
    let path = PathBuf::from(normalized);
    if !path.is_file() {
        return Err(AppError::new(
            ErrorCode::FileNotFound,
            "原 PDF 不在保存的位置，请使用“重新关联 PDF”选择迁移后的文件。",
        ));
    }
    if !path
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case("pdf"))
    {
        return Err(AppError::new(
            ErrorCode::PdfReadFailed,
            "只能打开 PDF 文件。",
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checks_missing_files_and_unicode_pdf_paths() {
        assert_eq!(
            existing_pdf("missing-paper.pdf").unwrap_err().code_string(),
            "FILE_NOT_FOUND"
        );
        let dir = std::env::temp_dir().join(format!("paper-file-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let file = dir.join("中文 论文.PDF");
        std::fs::write(&file, b"%PDF-1.4").unwrap();
        assert_eq!(existing_pdf(file.to_str().unwrap()).unwrap(), file);
        std::fs::remove_file(file).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
