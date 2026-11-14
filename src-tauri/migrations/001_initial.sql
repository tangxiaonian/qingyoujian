PRAGMA foreign_keys = ON;

CREATE TABLE favorite_folder (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    normalized_path TEXT NOT NULL CHECK (substr(normalized_path, 1, 1) = '/'),
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE (normalized_path)
);
CREATE INDEX idx_favorite_order ON favorite_folder(sort_order, id);

CREATE TABLE file_type (
    id TEXT PRIMARY KEY NOT NULL,
    type_key TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    extension TEXT NOT NULL CHECK (length(extension) > 0),
    default_basename TEXT NOT NULL CHECK (length(trim(default_basename)) > 0),
    initial_content TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    is_builtin INTEGER NOT NULL DEFAULT 1 CHECK (is_builtin IN (0, 1)),
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE menu_item (
    id TEXT PRIMARY KEY NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    sort_order INTEGER NOT NULL CHECK (sort_order >= 0),
    updated_at INTEGER NOT NULL
);

CREATE TABLE app_setting (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);