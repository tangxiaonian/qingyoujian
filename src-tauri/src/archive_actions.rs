use crate::domain::{archive, file_actions, Action, Result};
use crate::{finish_if_idle, platform, ACTIVE_ACTIONS};
use std::sync::atomic::Ordering;

/// Keep ZIP I/O off AppKit's thread and keep non-resident hosts alive until completion.
pub fn start(app: &tauri::AppHandle, action: &Action) -> Result<()> {
    let sources = file_actions::selection(&action.context)?;
    let operation = match action.action.as_str() {
        "compress_zip" => archive::Operation::Compress,
        "extract_zip_here" => archive::Operation::ExtractHere,
        _ => archive::Operation::ExtractFolder,
    };
    let text = if matches!(operation, archive::Operation::Compress) {
        "正在压缩 ZIP…"
    } else {
        "正在解压 ZIP…"
    };
    platform::feedback(text, false);
    let app = app.clone();
    ACTIVE_ACTIONS.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || {
        let report = archive::perform(&sources, operation);
        let text = report.message(operation);
        let failed = !report.errors.is_empty();
        let target = app.clone();
        let _ = app.run_on_main_thread(move || {
            if !report.outputs.is_empty() {
                platform::reveal_many(&report.outputs);
            }
            platform::feedback(&text, failed);
            ACTIVE_ACTIONS.fetch_sub(1, Ordering::Relaxed);
            finish_if_idle(&target);
        });
    });
    Ok(())
}
