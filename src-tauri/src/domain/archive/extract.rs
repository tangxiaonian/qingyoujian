use super::{is_zip, zip_error, AppError, Result};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use zip::ZipArchive;

/// Extract into an owned empty directory so a failed CRC or invalid entry cannot touch existing files.
pub(super) fn extract(source: &Path, separate: bool) -> Result<Vec<PathBuf>> {
    if !is_zip(source) {
        return Err(AppError::new(
            "ZIP_INVALID",
            "请选择普通 ZIP 文件；文件夹和其他格式无法解压。",
        ));
    }
    let parent = source
        .parent()
        .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "无法确定 ZIP 所在目录。"))?;
    let stage = tempfile::Builder::new()
        .prefix(".qyj-unzip-")
        .tempdir_in(parent)?;
    let mut archive = ZipArchive::new(File::open(source)?).map_err(zip_error)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        // Validate the original spelling too: this library can normalize rooted names into relative ones.
        let original = entry.name().map_err(zip_error)?;
        if original.starts_with(['/', '\\'])
            || original.as_bytes().get(1) == Some(&b':')
            || original.split(['/', '\\']).any(|part| part == "..")
        {
            return Err(AppError::new(
                "ZIP_INVALID_PATH",
                "ZIP 包含绝对路径或上级目录路径，已停止解压。",
            ));
        }
        let relative = entry
            .enclosed_name()
            .filter(|path| {
                !path.as_os_str().is_empty()
                    && path
                        .components()
                        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
            })
            .ok_or_else(|| {
                AppError::new("ZIP_INVALID_PATH", "ZIP 包含越界或无效路径，已停止解压。")
            })?;
        if entry.is_symlink()
            || entry.unix_mode().is_some_and(|mode| {
                let kind = mode & 0o170000;
                kind != 0 && kind != 0o100000 && kind != 0o040000
            })
        {
            return Err(AppError::new(
                "ZIP_UNSUPPORTED_ENTRY",
                "ZIP 含有符号链接或特殊文件，当前版本暂不支持解压。",
            ));
        }
        let target = stage.path().join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            // Repeated file paths inside the ZIP are invalid; never silently replace an earlier entry.
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            io::copy(&mut entry, &mut output).map_err(|error| {
                AppError::new(
                    "ZIP_READ_FAILED",
                    format!("ZIP 内容损坏或无法写入（也可能磁盘空间不足）：{error}"),
                )
            })?;
            if let Some(mode) = entry.unix_mode() {
                fs::set_permissions(&target, fs::Permissions::from_mode(mode & 0o777))?;
            }
        }
    }
    if separate {
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("解压文件");
        let mut destination = parent.join(name);
        let mut number = 2;
        while destination.symlink_metadata().is_ok() {
            destination = parent.join(format!("{name} {number}"));
            number += 1;
        }
        crate::platform::transfer(stage.path(), &destination, true)?;
        Ok(vec![destination])
    } else {
        let mut children =
            fs::read_dir(stage.path())?.collect::<std::result::Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        let mut published = Vec::new();
        for child in children {
            let destination = super::super::file_actions::available_target(&child.path(), parent)?;
            if let Err(error) = crate::platform::transfer(&child.path(), &destination, true) {
                return Err(AppError::new(
                    "ZIP_PUBLISH_FAILED",
                    format!(
                        "已有 {} 项解压成功，其余内容未写入。{}",
                        published.len(),
                        error.message
                    ),
                ));
            }
            published.push(destination);
        }
        Ok(published)
    }
}
