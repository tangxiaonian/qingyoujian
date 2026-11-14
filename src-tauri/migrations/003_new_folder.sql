UPDATE menu_item SET sort_order=sort_order+1;
INSERT INTO menu_item VALUES ('new-folder',1,0,CAST(strftime('%s','now') AS INTEGER)*1000);
UPDATE app_setting SET value_json=CAST(CAST(value_json AS INTEGER)+1 AS TEXT) WHERE key='settings_revision';
PRAGMA user_version=3;
