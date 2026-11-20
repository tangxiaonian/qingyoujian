use super::*;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub enum Operation {
    Copy,
    Move,
    Trash,
}
pub struct BatchReport {
    pub succeeded: usize,
    pub errors: Vec<String>,
}
impl BatchReport {
    /// Keep the completion message short while preserving per-item failures.
    pub fn message(&self, operation: Operation) -> String {
        let verb = match operation {
            Operation::Copy => "复制",
            Operation::Move => "移动",
            Operation::Trash => "移到废纸篓",
        };
        let mut message = format!("已{verb} {} 项", self.succeeded);
        if !self.errors.is_empty() {
            message.push_str(&format!(
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
        message
    }
}

/// File operations act on explicit selected items only, never on a container background.
pub fn selection(context: &Context) -> Result<Vec<PathBuf>> {
    if context.menu_kind != "items" || context.selected_paths.is_empty() {
        return Err(AppError::new(
            "CONTEXT_UNAVAILABLE",
            "请先选择要操作的文件或文件夹。",
        ));
    }
    context
        .selected_paths
        .iter()
        .map(|p| normalize_path(p))
        .collect()
}

/// A collision creates a sibling name; folders retain dots as part of the folder name.
pub(super) fn available_target(source: &Path, destination: &Path) -> Result<PathBuf> {
    let name = source
        .file_name()
        .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "不能对磁盘根目录执行此操作。"))?
        .to_string_lossy();
    let is_directory = std::fs::symlink_metadata(source)?.is_dir();
    let extension = if is_directory {
        None
    } else {
        Path::new(name.as_ref())
            .extension()
            .and_then(|s| s.to_str())
    };
    let base = Path::new(name.as_ref())
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&name);
    let mut candidate = destination.join(name.as_ref());
    let mut number = 2;
    while candidate.symlink_metadata().is_ok() {
        let next = if let Some(ext) = extension {
            format!("{base} {number}.{ext}")
        } else {
            format!("{name} {number}")
        };
        candidate = destination.join(next);
        number += 1;
    }
    Ok(candidate)
}

/// Validate normal user mistakes before calling the OS; never copy a directory into itself.
fn transfer_one(source: &Path, destination: &Path, move_item: bool) -> Result<()> {
    let parent = source
        .parent()
        .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "不能操作磁盘根目录。"))?;
    if move_item && parent.canonicalize()? == destination.canonicalize()? {
        return Err(AppError::new("INVALID_ARGUMENT", "已经在目标目录中。"));
    }
    if std::fs::symlink_metadata(source)?.is_dir()
        && destination
            .canonicalize()?
            .starts_with(source.canonicalize()?)
    {
        return Err(AppError::new(
            "INVALID_ARGUMENT",
            "不能把文件夹复制或移动到它自己或其子目录。",
        ));
    }
    let target = available_target(source, destination)?;
    crate::platform::transfer(source, &target, move_item)
}

/// Run outside the UI thread; successes remain successful if a later item fails.
pub fn perform(
    sources: &[PathBuf],
    destination: Option<&Path>,
    operation: Operation,
) -> BatchReport {
    let mut report = BatchReport {
        succeeded: 0,
        errors: vec![],
    };
    for source in sources {
        let outcome = match operation {
            Operation::Trash => crate::platform::trash(source).map(|_| ()),
            Operation::Copy | Operation::Move => destination
                .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "缺少目标目录。"))
                .and_then(|directory| {
                    transfer_one(source, directory, matches!(operation, Operation::Move))
                }),
        };
        match outcome {
            Ok(()) => report.succeeded += 1,
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
mod tests {
    use super::*;
    /// Exercise actual Foundation copies/moves, collision handling and invalid descendants with temp fixtures.
    #[test]
    fn copy_move_and_partial_failures() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let dest = temp.path().join("目标 目录");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir(&dest).unwrap();
        let file = source.join("文件.txt");
        std::fs::write(&file, "original").unwrap();
        std::fs::write(dest.join("文件.txt"), "keep").unwrap();
        let copied = perform(std::slice::from_ref(&file), Some(&dest), Operation::Copy);
        assert_eq!(copied.succeeded, 1);
        assert!(copied.errors.is_empty());
        assert_eq!(
            std::fs::read_to_string(dest.join("文件 2.txt")).unwrap(),
            "original"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("文件.txt")).unwrap(),
            "keep"
        );
        assert!(file.exists());
        let moved = perform(
            &[file.clone(), source.join("missing")],
            Some(&dest),
            Operation::Move,
        );
        assert_eq!(moved.succeeded, 1);
        assert_eq!(moved.errors.len(), 1);
        assert!(!file.exists());
        assert!(dest.join("文件 3.txt").exists());
        let folder = source.join("目录.v1");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("a"), "nested").unwrap();
        assert_eq!(
            perform(std::slice::from_ref(&folder), Some(&dest), Operation::Copy).succeeded,
            1
        );
        assert!(dest.join("目录.v1/a").exists());
        assert_eq!(
            perform(
                std::slice::from_ref(&source),
                Some(&folder),
                Operation::Copy
            )
            .succeeded,
            0
        );
        assert_eq!(
            perform(&[dest.join("文件.txt")], Some(&dest), Operation::Move).succeeded,
            0
        );
    }
    /// Only a self-created temporary file is trashed and then removed from its exact returned location.
    #[test]
    fn recoverable_trash_api() {
        let temp = tempfile::tempdir().unwrap();
        let sample = temp
            .path()
            .join(format!("qyj-trash-{}.txt", uuid::Uuid::new_v4()));
        std::fs::write(&sample, "test-only").unwrap();
        let actual = crate::platform::trash(&sample)
            .unwrap()
            .expect("system Trash should return its destination");
        assert!(!sample.exists());
        assert_eq!(std::fs::read_to_string(&actual).unwrap(), "test-only");
        std::fs::remove_file(actual).unwrap();
    }
}
