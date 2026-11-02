-- Down: remove building_id from periods
CREATE TABLE periods_old (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT NOT NULL,
  start_date TEXT NOT NULL,
  num_days INTEGER NOT NULL,
  is_current INTEGER NOT NULL DEFAULT 0
);
INSERT INTO periods_old SELECT id, name, start_date, num_days, is_current FROM periods;
DROP TABLE periods;
ALTER TABLE periods_old RENAME TO periods;
