use super::{zip_error, AppError, Result};
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

/// Stream files into a sibling temporary ZIP and publish only after the central directory is complete.
pub(super) fn compress(sources: &[PathBuf]) -> Result<PathBuf> {
    let parent = sources[0]
        .parent()
        .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "不能压缩磁盘根目录。"))?;
    if sources.iter().any(|path| path.parent() != Some(parent)) {
        return Err(AppError::new(
            "INVALID_ARGUMENT",
            "请选择同一文件夹中的项目一起压缩。",
        ));
    }
    let mut temporary = NamedTempFile::with_prefix_in(".qyj-zip-", parent)?;
    {
        let mut writer = ZipWriter::new(temporary.as_file_mut());
        for source in sources {
            for entry in WalkDir::new(source).follow_links(false).sort_by_file_name() {
                let entry = entry.map_err(|error| {
                    AppError::new("ZIP_READ_FAILED", format!("无法读取待压缩项目：{error}"))
                })?;
                append(&mut writer, entry.path(), parent)?;
            }
        }
        writer.finish().map_err(zip_error)?;
    }
    let name = if sources.len() == 1 {
        format!(
            "{}.zip",
            sources[0].file_name().unwrap_or_default().to_string_lossy()
        )
    } else {
        "归档.zip".into()
    };
    let mut number = 1;
    loop {
        let filename = if number == 1 {
            name.clone()
        } else {
            format!(
                "{} {number}.zip",
                name.strip_suffix(".zip").unwrap_or(&name)
            )
        };
        let path = parent.join(filename);
        match temporary.persist_noclobber(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temporary = error.file;
                number += 1;
            }
            Err(error) => return Err(error.error.into()),
        }
    }
}

/// Keep hidden items, empty directories and executable bits; never follow symbolic links outside the selection.
fn append(writer: &mut ZipWriter<&mut File>, path: &Path, parent: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || (!metadata.is_file() && !metadata.is_dir()) {
        return Err(AppError::new(
            "ZIP_UNSUPPORTED_ENTRY",
            "当前版本仅支持普通文件和文件夹，不能压缩符号链接或特殊文件。",
        ));
    }
    let relative = path
        .strip_prefix(parent)
        .map_err(|_| AppError::new("INVALID_ARGUMENT", "无法确定压缩项目的相对路径。"))?;
    let name = relative
        .to_str()
        .ok_or_else(|| AppError::new("ZIP_INVALID_NAME", "文件名不是有效的 Unicode，无法压缩。"))?;
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(metadata.permissions().mode() & 0o777)
        .large_file(metadata.len() >= u32::MAX as u64);
    if metadata.is_dir() {
        writer
            .add_directory(format!("{name}/"), options)
            .map_err(zip_error)?;
    } else {
        writer.start_file(name, options).map_err(zip_error)?;
        std::io::copy(&mut File::open(path)?, writer)?;
    }
    Ok(())
}
