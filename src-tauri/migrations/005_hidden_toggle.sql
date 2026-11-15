-- Preserve visibility and placement when merging the two legacy menu entries.
INSERT INTO menu_item (id,enabled,sort_order,updated_at)
SELECT 'toggle-hidden',MAX(enabled),MIN(sort_order),CAST(strftime('%s','now') AS INTEGER)*1000
FROM menu_item WHERE id IN ('show-hidden','hide-hidden');
DELETE FROM menu_item WHERE id IN ('show-hidden','hide-hidden');
UPDATE app_setting SET value_json=CAST(CAST(value_json AS INTEGER)+1 AS TEXT) WHERE key='settings_revision';
PRAGMA user_version=5;
