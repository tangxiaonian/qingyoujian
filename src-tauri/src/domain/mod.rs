pub mod archive;
pub mod file_actions;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, AppError>;
impl AppError {
    /// Keep native details out of default logs; the UI receives a concise actionable message.
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl From<rusqlite::Error> for AppError {
    /// Translate persistence failures at the storage boundary.
    fn from(_: rusqlite::Error) -> Self {
        Self::new("STORAGE_ERROR", "无法读写本机配置，请检查数据目录权限。")
    }
}
impl From<std::io::Error> for AppError {
    /// Distinguish expected filesystem failures for both native and web interfaces.
    fn from(error: std::io::Error) -> Self {
        let code = match error.kind() {
            std::io::ErrorKind::NotFound => "NOT_FOUND",
            std::io::ErrorKind::PermissionDenied => "PERMISSION_DENIED",
            _ => "OS_OPERATION_FAILED",
        };
        Self::new(code, format!("文件操作失败：{error}"))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Favorite {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub sort_order: i64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileType {
    pub id: String,
    pub display_name: String,
    pub extension: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MenuItem {
    pub id: String,
    pub enabled: bool,
    pub sort_order: i64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub terminal_bundle_id: String,
    pub menu_items: Vec<MenuItem>,
    pub show_menu_bar_icon: bool,
    pub start_at_login: bool,
    pub run_in_background: bool,
}
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub menu_kind: String,
    pub targeted_path: Option<String>,
    #[serde(default)]
    pub selected_paths: Vec<String>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub protocol_version: u32,
    pub request_id: String,
    pub action: String,
    #[serde(default)]
    pub context: Context,
    pub resource_id: Option<String>,
}

/// Normalize syntactic paths without resolving symlinks or changing filename case.
pub fn normalize_path(path: &str) -> Result<PathBuf> {
    if !Path::new(path).is_absolute() || path.contains('\0') {
        return Err(AppError::new("INVALID_ARGUMENT", "请选择绝对目录路径。"));
    }
    let mut out = PathBuf::from("/");
    for component in Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::ParentDir => {
                out.pop();
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Apply the same directory semantics to creation, terminal and favorites.
pub fn resolve_folder(context: &Context) -> Result<PathBuf> {
    let target = if context.menu_kind == "items" && context.selected_paths.len() == 1 {
        context.selected_paths.first()
    } else {
        context.targeted_path.as_ref()
    };
    let target = target
        .ok_or_else(|| AppError::new("CONTEXT_UNAVAILABLE", "当前位置没有可操作的实际目录。"))?;
    let path = normalize_path(target)?;
    let metadata = std::fs::metadata(&path)?;
    if metadata.is_dir() && !crate::platform::is_package(&path) {
        return Ok(path);
    }
    path.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::new("CONTEXT_UNAVAILABLE", "无法确定文件所在目录。"))
}

/// Refuse paths as filenames; only the explicitly chosen directory may be written.
pub fn validate_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') || name.contains('\0') || name == "." || name == ".." {
        return Err(AppError::new(
            "INVALID_ARGUMENT",
            "文件名不能为空，也不能包含 / 或空字符。",
        ));
    }
    Ok(name)
}

/// Atomic create-new semantics preserve existing files while providing Finder-style numbering.
pub fn create_file(directory: &Path, filename: &str, contents: &[u8]) -> Result<PathBuf> {
    use std::io::Write;
    let filename = validate_name(filename)?;
    let path = Path::new(filename);
    let extension = path.extension().and_then(|s| s.to_str());
    let base = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    let mut number = 1;
    loop {
        let candidate = if number == 1 {
            filename.to_string()
        } else if let Some(ext) = extension {
            format!("{base} {number}.{ext}")
        } else {
            format!("{filename} {number}")
        };
        let destination = directory.join(candidate);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(contents) {
                    drop(file);
                    let _ = std::fs::remove_file(&destination);
                    return Err(error.into());
                }
                return Ok(destination);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                number += 1;
            }
            Err(error) => {
                return Err(error.into());
            }
        }
    }
}

/// Create one folder without overwriting a file or directory of the same name.
pub fn create_folder(directory: &Path, name: &str) -> Result<PathBuf> {
    let name = validate_name(name)?;
    let mut number = 1;
    loop {
        let candidate = if number == 1 {
            name.to_string()
        } else {
            format!("{name} {number}")
        };
        let path = directory.join(candidate);
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => number += 1,
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Existing user data must survive duplicate names and binary template creation.
    #[test]
    fn creation_preserves_existing_and_templates() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("中文 a.docx"), b"original").unwrap();
        let template = include_bytes!("../../../assets/templates/blank.docx");
        let created = create_file(temp.path(), "中文 a.docx", template).unwrap();
        assert_eq!(created.file_name().unwrap(), "中文 a 2.docx");
        assert_eq!(std::fs::read(created).unwrap(), template);
        assert_eq!(
            std::fs::read(temp.path().join("中文 a.docx")).unwrap(),
            b"original"
        );
        assert!(create_file(temp.path(), "../escape", b"").is_err());
        assert!(create_file(temp.path(), " ", b"").is_err());
    }
    /// Dots stay in folder names; a same-name file or directory is never replaced.
    #[test]
    fn new_folder_collision() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("项目.v1"), "keep").unwrap();
        let first = create_folder(temp.path(), "项目.v1").unwrap();
        assert_eq!(first.file_name().unwrap(), "项目.v1 2");
        assert!(first.is_dir());
        assert_eq!(
            create_folder(temp.path(), "项目.v1")
                .unwrap()
                .file_name()
                .unwrap(),
            "项目.v1 3"
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("项目.v1")).unwrap(),
            "keep"
        );
        assert!(create_folder(temp.path(), "../outside").is_err());
    }

    /// Multi-select uses the browsing directory instead of arbitrarily selecting the first item.
    #[test]
    fn directory_rules() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.txt");
        std::fs::write(&file, "").unwrap();
        let path = temp.path().to_string_lossy().to_string();
        let context = Context {
            menu_kind: "items".into(),
            targeted_path: Some("/".into()),
            selected_paths: vec![file.to_string_lossy().into()],
        };
        assert_eq!(resolve_folder(&context).unwrap(), temp.path());
        let multi = Context {
            selected_paths: vec!["/invalid/1".into(), "/invalid/2".into()],
            targeted_path: Some(path.clone()),
            ..context
        };
        assert_eq!(resolve_folder(&multi).unwrap(), temp.path());
        assert_eq!(normalize_path(&(path + "/a/../")).unwrap(), temp.path());
        assert!(resolve_folder(&Context::default()).is_err());
    }
}
