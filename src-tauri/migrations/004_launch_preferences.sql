INSERT INTO app_setting VALUES ('show_menu_bar_icon','false',CAST(strftime('%s','now') AS INTEGER)*1000);
INSERT INTO app_setting VALUES ('start_at_login','false',CAST(strftime('%s','now') AS INTEGER)*1000);
INSERT INTO app_setting VALUES ('run_in_background','true',CAST(strftime('%s','now') AS INTEGER)*1000);
UPDATE app_setting SET value_json=CAST(CAST(value_json AS INTEGER)+1 AS TEXT) WHERE key='settings_revision';
PRAGMA user_version=4;
