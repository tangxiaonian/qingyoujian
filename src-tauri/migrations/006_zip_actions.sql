-- Append three independent ZIP actions without resetting existing menu choices or order.
INSERT INTO menu_item (id, enabled, sort_order, updated_at)
SELECT 'compress-zip', 1, COALESCE(MAX(sort_order), -1) + 1, CAST(strftime('%s','now') AS INTEGER)*1000 FROM menu_item;
INSERT INTO menu_item (id, enabled, sort_order, updated_at)
SELECT 'extract-zip-here', 1, MAX(sort_order) + 1, CAST(strftime('%s','now') AS INTEGER)*1000 FROM menu_item;
INSERT INTO menu_item (id, enabled, sort_order, updated_at)
SELECT 'extract-zip-folder', 1, MAX(sort_order) + 1, CAST(strftime('%s','now') AS INTEGER)*1000 FROM menu_item;
UPDATE app_setting SET value_json=CAST(CAST(value_json AS INTEGER)+1 AS TEXT) WHERE key='settings_revision';
PRAGMA user_version=6;
