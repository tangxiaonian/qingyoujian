UPDATE menu_item SET sort_order=sort_order+3 WHERE sort_order>=2;
INSERT INTO menu_item VALUES ('copy-to',1,2,CAST(strftime('%s','now') AS INTEGER)*1000);
INSERT INTO menu_item VALUES ('move-to',1,3,CAST(strftime('%s','now') AS INTEGER)*1000);
INSERT INTO menu_item VALUES ('trash',1,4,CAST(strftime('%s','now') AS INTEGER)*1000);
UPDATE app_setting SET value_json=CAST(CAST(value_json AS INTEGER)+1 AS TEXT) WHERE key='settings_revision';
PRAGMA user_version=2;
