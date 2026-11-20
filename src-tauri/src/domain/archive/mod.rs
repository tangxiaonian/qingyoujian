mod extract;
mod pack;

use super::{AppError, Result};
use std::path::{Path, PathBuf};
use zip::result::ZipError;

#[derive(Clone, Copy)]
pub enum Operation {
    Compress,
    ExtractHere,
    ExtractFolder,
}

pub struct Report {
    pub outputs: Vec<PathBuf>,
    pub succeeded: usize,
    pub errors: Vec<String>,
}

impl Report {
    /// Report completed archives separately from failures in the same selection.
    pub fn message(&self, operation: Operation) -> String {
        let verb = if matches!(operation, Operation::Compress) {
            "压缩"
        } else {
            "解压"
        };
        let mut text = format!("已{verb} {} 个 ZIP", self.succeeded);
        if !self.errors.is_empty() {
            text.push_str(&format!(
                "，失败 {} 项。\n{}",
                self.errors.len(),
                self.errors
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        text
    }
}

/// Only real .zip files can be expanded; an arbitrary extension never runs an external program.
pub fn is_zip(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        && std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// Keep password, unsupported compression and damaged archives distinct in native feedback.
fn zip_error(error: ZipError) -> AppError {
    match error {
        ZipError::Io(error) => error.into(),
        ZipError::UnsupportedArchive(ZipError::PASSWORD_REQUIRED) | ZipError::InvalidPassword => {
            AppError::new(
                "ZIP_PASSWORD_REQUIRED",
                "这个 ZIP 需要密码，当前版本暂不支持加密 ZIP。",
            )
        }
        ZipError::UnsupportedArchive(_) | ZipError::CompressionMethodNotSupported(_) => {
            AppError::new(
                "ZIP_UNSUPPORTED",
                "这个 ZIP 使用了当前版本不支持的压缩方式。",
            )
        }
        _ => AppError::new("ZIP_INVALID", "ZIP 已损坏或格式不完整，请重新获取压缩包。"),
    }
}

/// Batch decompression continues with remaining archives; compression combines the whole selection.
pub fn perform(sources: &[PathBuf], operation: Operation) -> Report {
    let mut report = Report {
        outputs: vec![],
        succeeded: 0,
        errors: vec![],
    };
    if sources.is_empty() {
        report.errors.push("请先选择要压缩或解压的文件。".into());
        return report;
    }
    if matches!(operation, Operation::Compress) {
        match pack::compress(sources) {
            Ok(path) => {
                report.outputs.push(path);
                report.succeeded = 1;
            }
            Err(error) => report.errors.push(error.message),
        }
        return report;
    }
    for source in sources {
        match extract::extract(source, matches!(operation, Operation::ExtractFolder)) {
            Ok(paths) => {
                report.outputs.extend(paths);
                report.succeeded += 1;
            }
            Err(error) => report.errors.push(format!(
                "{}：{}",
                source.file_name().unwrap_or_default().to_string_lossy(),
                error.message
            )),
        }
    }
    report
}

#[cfg(test)]
mod tests;
