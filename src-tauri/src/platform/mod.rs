use crate::domain::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::Path;
use std::process::Command;

unsafe extern "C" {
    fn qyj_publish(json: *const c_char) -> bool;
    fn qyj_prompt(
        title: *const c_char,
        suggestion: *const c_char,
        message: *const c_char,
    ) -> *mut c_char;
    fn qyj_alert(message: *const c_char);
    fn qyj_choose_folder() -> *mut c_char;
    fn qyj_missing_folder() -> i32;
    fn qyj_toggle_hidden() -> *mut c_char;
    fn qyj_open_folder(path: *const c_char) -> bool;
    fn qyj_reveal(path: *const c_char);
    fn qyj_reveal_many(json: *const c_char);
    fn qyj_is_package(path: *const c_char) -> bool;
    fn qyj_accessory(accessory: bool);
    fn free(pointer: *mut std::ffi::c_void);
}

/// C strings are needed only at the Objective-C bridge; reject embedded NULs.
fn cstring(text: &str) -> Result<CString> {
    CString::new(text).map_err(|_| AppError::new("INVALID_ARGUMENT", "文本中含有空字符。"))
}
/// Native owned strings are copied once and immediately released.
unsafe fn take(pointer: *mut c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    let text = CStr::from_ptr(pointer).to_string_lossy().into_owned();
    free(pointer.cast());
    Some(text)
}
/// Publish the immutable menu projection to the extension's permitted preference domain.
pub fn publish(snapshot: &serde_json::Value) -> Result<()> {
    let value = cstring(&snapshot.to_string())?;
    if unsafe { qyj_publish(value.as_ptr()) } {
        Ok(())
    } else {
        Err(AppError::new(
            "SNAPSHOT_PUBLISH_FAILED",
            "已保存，菜单尚未刷新。请在设置中重试刷新。",
        ))
    }
}
/// Package detection keeps creation out of .app and document bundles.
pub fn is_package(path: &Path) -> bool {
    cstring(&path.to_string_lossy())
        .map(|s| unsafe { qyj_is_package(s.as_ptr()) })
        .unwrap_or(false)
}
/// Present a naming dialog without constructing any WebView.
pub fn prompt(title: &str, suggestion: &str, message: &str) -> Result<Option<String>> {
    let (title, suggestion, message) = (cstring(title)?, cstring(suggestion)?, cstring(message)?);
    Ok(unsafe {
        take(qyj_prompt(
            title.as_ptr(),
            suggestion.as_ptr(),
            message.as_ptr(),
        ))
    })
}
/// UI errors are not logged with file paths or contents.
pub fn alert(message: &str) {
    if let Ok(message) = cstring(message) {
        unsafe { qyj_alert(message.as_ptr()) };
    }
}
/// Folder selection must run on the main AppKit thread.
pub fn choose_folder() -> Option<String> {
    unsafe { take(qyj_choose_folder()) }
}
/// Return the user's explicit recovery choice: relocate, remove, cancel.
pub fn missing_folder() -> i32 {
    unsafe { qyj_missing_folder() }
}
/// Switch Dock visibility only when a real settings window exists.
pub fn accessory(value: bool) {
    unsafe { qyj_accessory(value) };
}
/// Open the directory using its URL without shell interpolation.
pub fn open_folder(path: &Path) -> Result<()> {
    if !path.is_dir() {
        return Err(AppError::new("NOT_FOUND", "这个常用目录暂时不可用。"));
    }
    let path = cstring(&path.to_string_lossy())?;
    if unsafe { qyj_open_folder(path.as_ptr()) } {
        Ok(())
    } else {
        Err(AppError::new(
            "OS_OPERATION_FAILED",
            "Finder 无法打开这个目录。",
        ))
    }
}
/// Select the created file in Finder, preserving the user's normal default editor.
pub fn reveal(path: &Path) -> Result<()> {
    let path = cstring(&path.to_string_lossy())?;
    unsafe { qyj_reveal(path.as_ptr()) };
    Ok(())
}
/// Reveal successful archive outputs in one Finder request after background work completes.
pub fn reveal_many(paths: &[std::path::PathBuf]) {
    if let Ok(json) = cstring(&serde_json::json!(paths).to_string()) {
        unsafe { qyj_reveal_many(json.as_ptr()) };
    }
}
/// Terminal handles a directory argument; no cd command or user-controlled script is generated.
pub fn terminal(path: &Path) -> Result<()> {
    let status = Command::new("/usr/bin/open")
        .args(["-a", "Terminal"])
        .arg(path)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::new(
            "APP_UNAVAILABLE",
            "无法在目标目录打开系统 Terminal。",
        ))
    }
}
/// Toggle Finder's live display without writing stale preferences or restarting its process.
pub fn toggle_hidden() -> Result<()> {
    match unsafe { take(qyj_toggle_hidden()) } {
        Some(message) => Err(AppError::new("OS_OPERATION_FAILED", &message)),
        None => Ok(()),
    }
}
/// Query pluginkit rather than assuming installation means activation.
pub fn extension_status() -> String {
    match Command::new("/usr/bin/pluginkit")
        .args(["-m", "-i", "local.qingyoujian.extension"])
        .output()
    {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.lines().any(|l| l.trim_start().starts_with('+')) {
                "enabled"
            } else {
                "disabled"
            }
            .into()
        }
        _ => "unknown".into(),
    }
}

unsafe extern "C" {
    fn qyj_transfer(
        source: *const c_char,
        destination: *const c_char,
        move_item: bool,
    ) -> *mut c_char;
    fn qyj_trash(source: *const c_char, result: *mut *mut c_char) -> *mut c_char;
    fn qyj_feedback(message: *const c_char, failure: bool);
}

/// Delegate to the system file manager; a failed move never triggers a custom delete fallback.
pub fn transfer(source: &Path, destination: &Path, move_item: bool) -> Result<()> {
    let source = cstring(&source.to_string_lossy())?;
    let destination = cstring(&destination.to_string_lossy())?;
    match unsafe {
        take(qyj_transfer(
            source.as_ptr(),
            destination.as_ptr(),
            move_item,
        ))
    } {
        None => Ok(()),
        Some(message) => Err(AppError::new("OS_OPERATION_FAILED", message)),
    }
}

/// Return the actual Trash path so temporary integration fixtures can be cleaned precisely.
pub fn trash(source: &Path) -> Result<Option<std::path::PathBuf>> {
    let source = cstring(&source.to_string_lossy())?;
    let mut result = std::ptr::null_mut();
    if let Some(error) = unsafe { take(qyj_trash(source.as_ptr(), &mut result)) } {
        return Err(AppError::new("OS_OPERATION_FAILED", error));
    }
    Ok(unsafe { take(result) }.map(std::path::PathBuf::from))
}

/// Call on AppKit's main thread after background file operations complete.
pub fn feedback(message: &str, failure: bool) {
    if let Ok(message) = cstring(message) {
        unsafe { qyj_feedback(message.as_ptr(), failure) };
    }
}

unsafe extern "C" {
    fn qyj_set_tray_visible(visible: bool);
    fn qyj_login_status() -> i32;
    fn qyj_set_login(enabled: bool) -> *mut c_char;
}

/// Update the native menu-bar item on the main AppKit thread.
pub fn set_tray(visible: bool) {
    unsafe {
        qyj_set_tray_visible(visible);
    }
}
/// Values mirror SMAppServiceStatus, which remains authoritative over the stored desired choice.
pub fn login_status() -> &'static str {
    match unsafe { qyj_login_status() } {
        1 => "enabled",
        2 => "requiresApproval",
        0 => "disabled",
        _ => "unavailable",
    }
}
/// Do not fall back to modifying LaunchAgents or opening privileged settings on failure.
pub fn set_login(enabled: bool) -> Result<()> {
    match unsafe { take(qyj_set_login(enabled)) } {
        None => Ok(()),
        Some(message) => Err(AppError::new("OS_OPERATION_FAILED", message)),
    }
}

unsafe extern "C" {
    fn qyj_open_login_settings();
}
/// The user completes any OS-required approval in macOS itself.
pub fn open_login_settings() {
    unsafe {
        qyj_open_login_settings();
    }
}
