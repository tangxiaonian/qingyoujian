#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod domain;
mod platform;
mod storage;

use domain::*;
use serde_json::{json, Value};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
static BACKGROUND: AtomicBool = AtomicBool::new(true);
static ACTIVE_ACTIONS: AtomicUsize = AtomicUsize::new(0);
use tauri::{Emitter, Manager};

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
struct AppState {
    store: Mutex<storage::Store>,
}
unsafe extern "C" {
    fn qyj_install_handler(callback: extern "C" fn(*const c_char));
    fn qyj_host_ready();
}

/// Keep the single SQLite connection inside the host; poisoned state is reported, not unwrapped.
fn store(state: &AppState) -> Result<std::sync::MutexGuard<'_, storage::Store>> {
    state
        .store
        .lock()
        .map_err(|_| AppError::new("STORAGE_ERROR", "配置服务不可用，请重新启动轻右键。"))
}

/// Non-resident mode waits for current actions and destroys no work in progress.
fn finish_if_idle(app: &tauri::AppHandle) {
    if BACKGROUND.load(Ordering::Relaxed) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(2200));
        let target = app.clone();
        let _ = app.run_on_main_thread(move || {
            if !BACKGROUND.load(Ordering::Relaxed)
                && ACTIVE_ACTIONS.load(Ordering::Relaxed) == 0
                && target.get_webview_window("settings").is_none()
            {
                target.exit(0);
            }
        });
    });
}

/// Save/refresh projects the committed state and notifies the existing settings UI.
fn publish(app: &tauri::AppHandle, store: &storage::Store) -> Result<()> {
    let snapshot = store.snapshot()?;
    platform::publish(&snapshot)?;
    let _ = app.emit("settings_changed", json!({"revision":snapshot["revision"]}));
    Ok(())
}

/// Create settings on demand; closing this window destroys its WebView.
fn show_settings(app: &tauri::AppHandle) -> Result<()> {
    platform::accessory(false);
    if let Some(window) = app.get_webview_window("settings") {
        window
            .show()
            .map_err(|e| AppError::new("OS_OPERATION_FAILED", e.to_string()))?;
        let _ = window.set_focus();
        return Ok(());
    }
    tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App("index.html".into()))
        .title("轻右键")
        .inner_size(930.0, 680.0)
        .min_inner_size(780.0, 560.0)
        .build()
        .map_err(|e| AppError::new("OS_OPERATION_FAILED", e.to_string()))?;
    Ok(())
}

/// Use current persisted favorites, offering native recovery when their directory disappears.
fn open_saved(state: &AppState, id: &str) -> Result<()> {
    let favorite = store(state)?.favorite(id)?;
    if Path::new(&favorite.path).is_dir() {
        return platform::open_folder(Path::new(&favorite.path));
    }
    match platform::missing_folder() {
        0 => {
            if let Some(path) = platform::choose_folder() {
                let favorite = store(state)?.relocate(id, &path)?;
                platform::open_folder(Path::new(&favorite.path))?;
            }
        }
        1 => {
            store(state)?.remove(id)?;
        }
        _ => {}
    }
    Ok(())
}

/// Execute only known Finder actions after the native bridge authenticates the sending extension.
fn dispatch(app: &tauri::AppHandle, action: Action) -> Result<()> {
    if action.protocol_version != 1 {
        return Err(AppError::new(
            "UNSUPPORTED_PROTOCOL",
            "请重新安装同一版本的应用与扩展。",
        ));
    }
    if action.request_id.is_empty() {
        return Err(AppError::new("INVALID_ARGUMENT", "动作缺少请求编号。"));
    }
    if action.action == "show_settings" {
        return show_settings(app);
    }
    let state = app.state::<AppState>();
    match action.action.as_str() {
        "new_folder" => {
            let directory = resolve_folder(&action.context)?;
            if let Some(name) = platform::prompt(
                "新建文件夹",
                "新建文件夹",
                &format!("位置：{}", directory.display()),
            )? {
                let folder = create_folder(&directory, &name)?;
                platform::reveal(&folder)?;
            }
        }
        "new_file" => {
            let directory = resolve_folder(&action.context)?;
            let (kind, content) = {
                let db = store(&state)?;
                let kind = action
                    .resource_id
                    .as_ref()
                    .map(|id| {
                        db.types().and_then(|types| {
                            types
                                .into_iter()
                                .find(|t| &t.id == id)
                                .ok_or_else(|| AppError::new("NOT_FOUND", "文件类型不可用。"))
                        })
                    })
                    .transpose()?;
                let content = kind
                    .as_ref()
                    .map(|t| db.content(&t.id))
                    .transpose()?
                    .unwrap_or_default();
                (kind, content)
            };
            let suggestion = kind
                .as_ref()
                .map(|t| format!("未命名.{}", t.extension))
                .unwrap_or("未命名.txt".into());
            if let Some(mut name) = platform::prompt(
                "新建文件",
                &suggestion,
                &format!("位置：{}", directory.display()),
            )? {
                name = validate_name(&name)?.to_string();
                if let Some(kind) = &kind {
                    if !name.ends_with(&format!(".{}", kind.extension)) {
                        name = format!("{name}.{}", kind.extension);
                    }
                }
                let file = create_file(&directory, &name, &content)?;
                platform::reveal(&file)?;
            }
        }
        "copy_items" | "move_items" | "trash_items" => {
            use domain::file_actions::{perform, selection, Operation};
            let sources = selection(&action.context)?;
            let operation = match action.action.as_str() {
                "copy_items" => Operation::Copy,
                "move_items" => Operation::Move,
                _ => Operation::Trash,
            };
            let destination = if matches!(operation, Operation::Trash) {
                None
            } else if let Some(id) = action.resource_id {
                let favorite = store(&state)?.favorite(&id)?;
                let path = domain::normalize_path(&favorite.path)?;
                if !path.is_dir() {
                    return Err(AppError::new(
                        "NOT_FOUND",
                        "目标收藏目录不可用，请先在设置中重新定位。",
                    ));
                }
                Some(path)
            } else {
                match platform::choose_folder() {
                    Some(path) => Some(domain::normalize_path(&path)?),
                    None => return Ok(()),
                }
            };
            let app = app.clone();
            ACTIVE_ACTIONS.fetch_add(1, Ordering::Relaxed);
            std::thread::spawn(move || {
                let report = perform(&sources, destination.as_deref(), operation);
                let text = report.message(operation);
                let failure = !report.errors.is_empty();
                let target = app.clone();
                let _ = app.run_on_main_thread(move || {
                    platform::feedback(&text, failure);
                    ACTIVE_ACTIONS.fetch_sub(1, Ordering::Relaxed);
                    finish_if_idle(&target);
                });
            });
        }
        "open_terminal" => platform::terminal(&resolve_folder(&action.context)?)?,
        "set_hidden_visible" => platform::set_hidden(
            action
                .visible
                .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "缺少显示状态。"))?,
        )?,
        "add_favorite" => {
            let directory = resolve_folder(&action.context)?;
            let created = {
                let mut db = store(&state)?;
                let (_, created) = db.add(&directory.to_string_lossy(), None)?;
                publish(app, &db)?;
                created
            };
            if !created {
                platform::alert("这个目录已经在常用目录中。");
            }
        }
        "open_favorite" => {
            let id = action
                .resource_id
                .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "缺少收藏编号。"))?;
            open_saved(&state, &id)?;
            publish(app, &*store(&state)?)?;
        }
        _ => return Err(AppError::new("INVALID_ARGUMENT", "不支持的右键操作。")),
    }
    Ok(())
}

/// The callback is synchronous within AppKit's trusted Apple Event handler.
extern "C" fn receive_action(raw: *const c_char) {
    if raw.is_null() {
        return;
    }
    if let Some(app) = APP.get() {
        ACTIVE_ACTIONS.fetch_add(1, Ordering::Relaxed);
        let result = (|| {
            let text = unsafe { CStr::from_ptr(raw) }.to_string_lossy();
            if text == "qingyoujian-internal:quit" {
                app.exit(0);
                return Ok(());
            }
            if text == "qingyoujian-internal:settings" {
                return show_settings(app);
            }
            let url = url::Url::parse(&text)
                .map_err(|_| AppError::new("INVALID_ARGUMENT", "无效动作链接。"))?;
            if url.scheme() != "qingyoujian" || url.host_str() != Some("action") {
                return Err(AppError::new("INVALID_ARGUMENT", "无效动作入口。"));
            }
            let payload = url
                .query_pairs()
                .find(|(key, _)| key == "payload")
                .ok_or_else(|| AppError::new("INVALID_ARGUMENT", "动作内容为空。"))?
                .1
                .into_owned();
            use base64::Engine;
            let data = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(payload)
                .map_err(|_| AppError::new("INVALID_ARGUMENT", "动作编码错误。"))?;
            let action: Action = serde_json::from_slice(&data)
                .map_err(|_| AppError::new("INVALID_ARGUMENT", "动作格式错误。"))?;
            dispatch(app, action)
        })();
        if let Err(error) = result {
            platform::alert(&error.message);
        }
        ACTIVE_ACTIONS.fetch_sub(1, Ordering::Relaxed);
        finish_if_idle(app);
    }
}

#[tauri::command]
/// An explicit quit exits the lightweight host; closing settings alone preserves Finder actions.
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
/// Return actual registration status alongside the persisted settings.
fn get_app_state(state: tauri::State<AppState>) -> Result<Value> {
    let db = store(&state)?;
    Ok(
        json!({"version":env!("CARGO_PKG_VERSION"),"extensionStatus":platform::extension_status(),"loginStatus":platform::login_status(),"settings":db.settings()?,"fileTypes":db.types()?,"favorites":db.favorites()?}),
    )
}
#[tauri::command]
/// Read supported application settings without querying Finder itself.
fn get_settings(state: tauri::State<AppState>) -> Result<AppSettings> {
    store(&state)?.settings()
}
#[tauri::command]
/// File types expose metadata only; document bytes stay inside the Rust host.
fn list_file_types(state: tauri::State<AppState>) -> Result<Vec<FileType>> {
    store(&state)?.types()
}
#[tauri::command]
/// Return favorites in their saved order.
fn list_favorites(state: tauri::State<AppState>) -> Result<Vec<Favorite>> {
    store(&state)?.favorites()
}
#[tauri::command]
/// Add the user-selected directory and refresh Finder's menu snapshot.
fn add_favorite(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    path: String,
    display_name: Option<String>,
) -> Result<Value> {
    let mut db = store(&state)?;
    let (favorite, created) = db.add(&path, display_name.as_deref())?;
    publish(&app, &db)?;
    Ok(json!({"favorite":favorite,"created":created}))
}
#[tauri::command]
/// Rename only a favorite's display label.
fn rename_favorite(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    id: String,
    display_name: String,
) -> Result<Favorite> {
    let mut db = store(&state)?;
    let favorite = db.rename(&id, &display_name)?;
    publish(&app, &db)?;
    Ok(favorite)
}
#[tauri::command]
/// Forget a directory without touching its files.
fn remove_favorite(app: tauri::AppHandle, state: tauri::State<AppState>, id: String) -> Result<()> {
    let mut db = store(&state)?;
    db.remove(&id)?;
    publish(&app, &db)
}
#[tauri::command]
/// Apply the complete new favorite order atomically.
fn reorder_favorites(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    ids: Vec<String>,
) -> Result<Vec<Favorite>> {
    let mut db = store(&state)?;
    let result = db.reorder(&ids)?;
    publish(&app, &db)?;
    Ok(result)
}
#[tauri::command]
/// Save a replacement location chosen by the user.
fn relocate_favorite(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    id: String,
    path: String,
) -> Result<Favorite> {
    let mut db = store(&state)?;
    let result = db.relocate(&id, &path)?;
    publish(&app, &db)?;
    Ok(result)
}
#[tauri::command]
/// The web settings interface offers its own recovery controls on missing paths.
fn open_favorite(state: tauri::State<AppState>, id: String) -> Result<()> {
    let favorite = store(&state)?.favorite(&id)?;
    platform::open_folder(Path::new(&favorite.path))
}
#[tauri::command]
/// Frontend settings contain only the supported menu whitelist and Terminal bundle id.
fn update_settings(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    patch: AppSettings,
) -> Result<AppSettings> {
    let os_enabled = matches!(platform::login_status(), "enabled" | "requiresApproval");
    if patch.start_at_login != os_enabled {
        platform::set_login(patch.start_at_login)?;
    }
    let mut db = store(&state)?;
    let result = db.update_settings(&patch)?;
    BACKGROUND.store(result.run_in_background, Ordering::Relaxed);
    let visible = result.show_menu_bar_icon;
    app.run_on_main_thread(move || platform::set_tray(visible))
        .map_err(|e| AppError::new("OS_OPERATION_FAILED", e.to_string()))?;
    publish(&app, &db)?;
    Ok(result)
}
#[tauri::command]
/// Retry projection publishing independently of database persistence.
fn refresh_menu_snapshot(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<Value> {
    let db = store(&state)?;
    publish(&app, &db)?;
    Ok(json!({"revision":db.snapshot()?["revision"]}))
}
#[tauri::command]
/// User-facing approval shortcut for macOS login-item authorization.
fn open_login_settings() {
    platform::open_login_settings();
}

#[tauri::command]
/// Open system settings; never silently grant extension or privacy permissions.
fn open_extension_settings() -> Result<()> {
    std::process::Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.ExtensionsPreferences")
        .status()?;
    Ok(())
}
#[tauri::command]
/// Dispatch AppKit's modal folder picker to the main thread from a Tauri worker command.
async fn choose_directory(app: tauri::AppHandle) -> Result<Option<String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(platform::choose_folder());
    })
    .map_err(|e| AppError::new("OS_OPERATION_FAILED", e.to_string()))?;
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv()
            .map_err(|_| AppError::new("OS_OPERATION_FAILED", "目录选择器已关闭。"))
    })
    .await
    .map_err(|_| AppError::new("OS_OPERATION_FAILED", "目录选择失败。"))?
}

/// No WebView is created until the user explicitly asks for the settings page.
fn main() {
    if std::env::args().any(|arg| arg == "--diagnostic-login-status") {
        println!("{}", platform::login_status());
        return;
    }
    if std::env::args().any(|arg| arg == "--diagnostic-login-roundtrip") {
        let before = platform::login_status();
        println!("before: {before}");
        if before != "disabled" && before != "unavailable" {
            println!("Skipped: preserving an existing or unavailable OS registration.");
            return;
        }
        match platform::set_login(true) {
            Ok(()) => {
                println!("registered: {}", platform::login_status());
                if let Err(error) = platform::set_login(false) {
                    eprintln!("restore failed: {}", error.message);
                    std::process::exit(1);
                }
                println!("restored: {}", platform::login_status());
            }
            Err(error) => {
                eprintln!("registration failed: {}", error.message);
                std::process::exit(1);
            }
        }
        return;
    }
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let db = match storage::Store::open(
        &home.join("Library/Application Support/local.qingyoujian/qingyoujian.sqlite3"),
    ) {
        Ok(db) => db,
        Err(error) => {
            eprintln!("{}", error.code);
            return;
        }
    };
    unsafe {
        qyj_install_handler(receive_action);
    }
    let app = tauri::Builder::default()
        .manage(AppState {
            store: Mutex::new(db),
        })
        .invoke_handler(tauri::generate_handler![
            quit_app,
            get_app_state,
            get_settings,
            list_file_types,
            list_favorites,
            add_favorite,
            rename_favorite,
            remove_favorite,
            reorder_favorites,
            relocate_favorite,
            open_favorite,
            update_settings,
            refresh_menu_snapshot,
            open_extension_settings,
            open_login_settings,
            choose_directory
        ])
        .setup(|app| {
            let _ = APP.set(app.handle().clone());
            platform::accessory(true);
            unsafe {
                qyj_install_handler(receive_action);
            };
            let state = app.state::<AppState>();
            if let Ok(db) = store(&state) {
                if let Ok(settings) = db.settings() {
                    BACKGROUND.store(settings.run_in_background, Ordering::Relaxed);
                    platform::set_tray(settings.show_menu_bar_icon);
                }
                if let Err(error) = publish(app.handle(), &db) {
                    eprintln!("{}", error.code);
                }
            }
            unsafe {
                qyj_host_ready();
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("无法初始化应用运行环境");
    app.run(|app, event| match event {
        tauri::RunEvent::Ready => finish_if_idle(app),
        tauri::RunEvent::Reopen { .. } => {
            if let Err(error) = show_settings(app) {
                platform::alert(&error.message);
            }
        }
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::Destroyed,
            ..
        } => {
            platform::accessory(true);
            finish_if_idle(app);
        }
        tauri::RunEvent::ExitRequested {
            api, code: None, ..
        } if BACKGROUND.load(Ordering::Relaxed) || ACTIVE_ACTIONS.load(Ordering::Relaxed) > 0 => {
            api.prevent_exit();
        }
        _ => {}
    });
}
