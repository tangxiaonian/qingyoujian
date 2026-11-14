use crate::domain::*;
use rusqlite::{params, Connection};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Store {
    db: Connection,
}

/// Timestamp values are storage metadata, not UI clock strings.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Store {
    /// Open a local database and migrate once; never recreate existing user configuration.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut db = Connection::open(path)?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 4 {
            return Err(AppError::new(
                "STORAGE_ERROR",
                "配置来自较新版本，请使用更新的轻右键。",
            ));
        }
        if version == 0 {
            let tx = db.transaction()?;
            tx.execute_batch(include_str!("../../migrations/001_initial.sql"))?;
            let menu = [
                "new-file",
                "copy-path",
                "terminal",
                "show-hidden",
                "hide-hidden",
                "favorites",
                "add-favorite",
            ];
            for (index, id) in menu.iter().enumerate() {
                tx.execute(
                    "INSERT INTO menu_item VALUES(?1,1,?2,?3)",
                    params![id, index as i64, now()],
                )?;
            }
            for (index, (id, key, title, ext, content)) in [
                ("builtin-txt", "txt", "TXT 文本", "txt", ""),
                ("builtin-md", "markdown", "Markdown", "md", ""),
                ("builtin-json", "json", "JSON", "json", "{}\n"),
                (
                    "builtin-xml",
                    "xml",
                    "XML",
                    "xml",
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root/>\n",
                ),
                ("builtin-sh", "shell", "Shell 脚本", "sh", "#!/bin/sh\n"),
                ("builtin-docx", "word", "Word 文档", "docx", ""),
                ("builtin-xlsx", "excel", "Excel 表格", "xlsx", ""),
            ]
            .iter()
            .enumerate()
            {
                tx.execute(
                    "INSERT INTO file_type VALUES(?1,?2,?3,?4,'未命名',?5,1,1,?6,?7,?7)",
                    params![id, key, title, ext, content, index as i64, now()],
                )?;
            }
            tx.execute(
                "INSERT INTO app_setting VALUES('terminal_bundle_id','\"com.apple.Terminal\"',?1)",
                [now()],
            )?;
            tx.execute(
                "INSERT INTO app_setting VALUES('settings_revision','1',?1)",
                [now()],
            )?;
            tx.execute_batch("PRAGMA user_version = 1")?;
            tx.commit()?;
        }
        if version < 2 {
            let tx = db.transaction()?;
            tx.execute_batch(include_str!("../../migrations/002_file_actions.sql"))?;
            tx.commit()?;
        }
        if version < 3 {
            let tx = db.transaction()?;
            tx.execute_batch(include_str!("../../migrations/003_new_folder.sql"))?;
            tx.commit()?;
        }
        if version < 4 {
            let tx = db.transaction()?;
            tx.execute_batch(include_str!("../../migrations/004_launch_preferences.sql"))?;
            tx.commit()?;
        }
        Ok(Self { db })
    }

    /// Stable ids, rather than mutable menu positions, identify favorite actions.
    pub fn favorites(&self) -> Result<Vec<Favorite>> {
        let mut stmt = self.db.prepare("SELECT id, display_name, normalized_path, sort_order FROM favorite_folder ORDER BY sort_order,id")?;
        let rows = stmt.query_map([], |r| {
            Ok(Favorite {
                id: r.get(0)?,
                display_name: r.get(1)?,
                path: r.get(2)?,
                sort_order: r.get(3)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Re-read at invocation so relocating a favorite does not leave stale menu targets.
    pub fn favorite(&self, id: &str) -> Result<Favorite> {
        self.favorites()?
            .into_iter()
            .find(|f| f.id == id)
            .ok_or_else(|| AppError::new("NOT_FOUND", "该收藏已被移除。"))
    }

    /// A directory is checked only when added, relocated or opened, never per menu construction.
    pub fn add(&mut self, path: &str, name: Option<&str>) -> Result<(Favorite, bool)> {
        let path = crate::domain::normalize_path(path)?;
        if !path.is_dir() || crate::platform::is_package(&path) {
            return Err(AppError::new("NOT_FOUND", "请选择可用的普通文件夹。"));
        }
        let path = path.to_string_lossy().to_string();
        let existing = self.favorites()?;
        if let Some(found) = existing.iter().find(|f| f.path == path) {
            return Ok((found.clone(), false));
        }
        let default = Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("根目录");
        let display = validate_name(name.unwrap_or(default))?;
        let item = Favorite {
            id: Uuid::new_v4().to_string(),
            display_name: display.into(),
            path,
            sort_order: existing.last().map(|f| f.sort_order + 1).unwrap_or(0),
        };
        let tx = self.db.transaction()?;
        tx.execute(
            "INSERT INTO favorite_folder VALUES(?1,?2,?3,?4,?5,?5)",
            params![
                item.id,
                item.display_name,
                item.path,
                item.sort_order,
                now()
            ],
        )?;
        Self::bump(&tx)?;
        tx.commit()?;
        Ok((item, true))
    }

    /// Rename a bookmark label; the on-disk directory name is never touched.
    pub fn rename(&mut self, id: &str, name: &str) -> Result<Favorite> {
        let name = validate_name(name)?;
        self.favorite(id)?;
        let tx = self.db.transaction()?;
        tx.execute(
            "UPDATE favorite_folder SET display_name=?1,updated_at=?2 WHERE id=?3",
            params![name, now(), id],
        )?;
        Self::bump(&tx)?;
        tx.commit()?;
        self.favorite(id)
    }

    /// Relocation updates only the saved path, preserving the action's stable id.
    pub fn relocate(&mut self, id: &str, path: &str) -> Result<Favorite> {
        self.favorite(id)?;
        let path = normalize_path(path)?;
        if !path.is_dir() || crate::platform::is_package(&path) {
            return Err(AppError::new("NOT_FOUND", "所选目录不可用。"));
        }
        let path = path.to_string_lossy().to_string();
        if self
            .favorites()?
            .iter()
            .any(|f| f.id != id && f.path == path)
        {
            return Err(AppError::new("ALREADY_EXISTS", "这个目录已经在收藏中。"));
        }
        let tx = self.db.transaction()?;
        tx.execute(
            "UPDATE favorite_folder SET normalized_path=?1,updated_at=?2 WHERE id=?3",
            params![path, now(), id],
        )?;
        Self::bump(&tx)?;
        tx.commit()?;
        self.favorite(id)
    }

    /// Delete a reference, with no filesystem call.
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM favorite_folder WHERE id=?1", [id])?;
        Self::bump(&tx)?;
        tx.commit()?;
        Ok(())
    }

    /// Ordering changes are a single transaction and must contain the full current id set.
    pub fn reorder(&mut self, ids: &[String]) -> Result<Vec<Favorite>> {
        let old = self.favorites()?;
        let supplied: std::collections::HashSet<_> = ids.iter().collect();
        if ids.len() != old.len()
            || supplied.len() != ids.len()
            || old.iter().any(|f| !supplied.contains(&f.id))
        {
            return Err(AppError::new(
                "INVALID_ARGUMENT",
                "收藏列表已变化，请刷新后重新排序。",
            ));
        }
        let tx = self.db.transaction()?;
        for (index, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE favorite_folder SET sort_order=?1,updated_at=?2 WHERE id=?3",
                params![index as i64, now(), id],
            )?;
        }
        Self::bump(&tx)?;
        tx.commit()?;
        self.favorites()
    }

    /// Supported actions are seeded internally; clients cannot add executable actions.
    pub fn settings(&self) -> Result<AppSettings> {
        let mut stmt = self
            .db
            .prepare("SELECT id,enabled,sort_order FROM menu_item ORDER BY sort_order,id")?;
        let rows = stmt.query_map([], |r| {
            Ok(MenuItem {
                id: r.get(0)?,
                enabled: r.get(1)?,
                sort_order: r.get(2)?,
            })
        })?;
        let read_bool = |key: &str| -> Result<bool> {
            let value: String = self.db.query_row(
                "SELECT value_json FROM app_setting WHERE key=?1",
                [key],
                |r| r.get(0),
            )?;
            Ok(value == "true")
        };
        Ok(AppSettings {
            show_menu_bar_icon: read_bool("show_menu_bar_icon")?,
            start_at_login: read_bool("start_at_login")?,
            run_in_background: read_bool("run_in_background")?,
            terminal_bundle_id: "com.apple.Terminal".into(),
            menu_items: rows.collect::<std::result::Result<Vec<_>, _>>()?,
        })
    }

    /// MVP supports only Terminal; validate menu ids against the database whitelist.
    pub fn update_settings(&mut self, settings: &AppSettings) -> Result<AppSettings> {
        let old = self.settings()?;
        let ids: std::collections::HashSet<_> = settings.menu_items.iter().map(|m| &m.id).collect();
        if settings.terminal_bundle_id != "com.apple.Terminal"
            || ids.len() != old.menu_items.len()
            || settings.menu_items.len() != ids.len()
            || old.menu_items.iter().any(|m| !ids.contains(&m.id))
        {
            return Err(AppError::new(
                "INVALID_ARGUMENT",
                "不支持的终端或菜单配置。",
            ));
        }
        let tx = self.db.transaction()?;
        for (key, value) in [
            ("show_menu_bar_icon", settings.show_menu_bar_icon),
            ("start_at_login", settings.start_at_login),
            ("run_in_background", settings.run_in_background),
        ] {
            tx.execute(
                "UPDATE app_setting SET value_json=?1,updated_at=?2 WHERE key=?3",
                params![value.to_string(), now(), key],
            )?;
        }
        for (index, item) in settings.menu_items.iter().enumerate() {
            tx.execute(
                "UPDATE menu_item SET enabled=?1,sort_order=?2,updated_at=?3 WHERE id=?4",
                params![item.enabled, index as i64, now(), item.id],
            )?;
        }
        Self::bump(&tx)?;
        tx.commit()?;
        self.settings()
    }

    /// File contents stay in the host; extension snapshots contain only type metadata.
    pub fn types(&self) -> Result<Vec<FileType>> {
        let mut stmt = self.db.prepare("SELECT id,display_name,extension FROM file_type WHERE enabled=1 ORDER BY sort_order,id")?;
        let rows = stmt.query_map([], |r| {
            Ok(FileType {
                id: r.get(0)?,
                display_name: r.get(1)?,
                extension: r.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Office templates are real OOXML archives embedded in the binary, not zero-byte placeholders.
    pub fn content(&self, id: &str) -> Result<Vec<u8>> {
        match id {
            "builtin-docx" => Ok(include_bytes!("../../../assets/templates/blank.docx").to_vec()),
            "builtin-xlsx" => Ok(include_bytes!("../../../assets/templates/blank.xlsx").to_vec()),
            _ => {
                let text: String = self.db.query_row(
                    "SELECT initial_content FROM file_type WHERE id=?1 AND enabled=1",
                    [id],
                    |r| r.get(0),
                )?;
                Ok(text.into_bytes())
            }
        }
    }

    /// Publish a committed read model; failure is distinguishable from a failed database write.
    pub fn snapshot(&self) -> Result<serde_json::Value> {
        let revision: i64 = self.db.query_row(
            "SELECT CAST(value_json AS INTEGER) FROM app_setting WHERE key='settings_revision'",
            [],
            |r| r.get(0),
        )?;
        Ok(
            serde_json::json!({"schemaVersion":1,"revision":revision,"menuItems":self.settings()?.menu_items,"fileTypes":self.types()?,"favorites":self.favorites()?}),
        )
    }

    /// Increment inside the caller's transaction so data and its revision are inseparable.
    fn bump(tx: &rusqlite::Transaction<'_>) -> Result<()> {
        tx.execute("UPDATE app_setting SET value_json=CAST(CAST(value_json AS INTEGER)+1 AS TEXT),updated_at=?1 WHERE key='settings_revision'",[now()])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// User launch preferences survive restart without changing actual macOS login registration.
    #[test]
    fn launch_preferences_persist() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("settings.sqlite3");
        let mut store = Store::open(&path).unwrap();
        let mut settings = store.settings().unwrap();
        assert!(
            !settings.show_menu_bar_icon && !settings.start_at_login && settings.run_in_background
        );
        settings.show_menu_bar_icon = true;
        settings.start_at_login = true;
        settings.run_in_background = false;
        store.update_settings(&settings).unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        let saved = store.settings().unwrap();
        assert!(saved.show_menu_bar_icon && saved.start_at_login && !saved.run_in_background);
        assert_eq!(saved.menu_items.len(), 11);
    }

    /// Verify persisted edits and deduplication without touching the user's actual database.
    #[test]
    fn favorite_lifecycle_and_restart() {
        let temp = tempfile::tempdir().unwrap();
        let db = temp.path().join("test.sqlite3");
        let directory = temp.path().join("项目 a");
        std::fs::create_dir(&directory).unwrap();
        let path = directory.to_str().unwrap();
        let mut store = Store::open(&db).unwrap();
        let (a, created) = store.add(path, None).unwrap();
        assert!(created);
        assert!(!store.add(&(path.to_string() + "/"), None).unwrap().1);
        store.rename(&a.id, "新名称").unwrap();
        assert!(directory.exists());
        let b = store
            .add(temp.path().to_str().unwrap(), Some("另一个"))
            .unwrap()
            .0;
        assert!(store.reorder(&[a.id.clone(), a.id.clone()]).is_err());
        store.reorder(&[b.id.clone(), a.id.clone()]).unwrap();
        drop(store);
        let mut store = Store::open(&db).unwrap();
        assert_eq!(store.favorites().unwrap()[0].id, b.id);
        assert_eq!(store.favorite(&a.id).unwrap().display_name, "新名称");
        assert!(store
            .relocate(&a.id, temp.path().to_str().unwrap())
            .is_err());
        store.remove(&a.id).unwrap();
        assert!(directory.is_dir());
        assert_eq!(store.types().unwrap().len(), 7);
        assert!(store.content("builtin-xlsx").unwrap().starts_with(b"PK"));
    }
}
