-- Add building_id to periods, assign existing periods to building 1, clean up duplicates
ALTER TABLE periods ADD COLUMN building_id INTEGER REFERENCES buildings(id);

-- Backfill existing periods to building 1 (the seed building "Лоджия 4")
UPDATE periods SET building_id = 1 WHERE building_id IS NULL;

-- Remove duplicates: keep only the latest period per (building_id, start_date, num_days)
DELETE FROM assignments WHERE period_id IN (
  SELECT p1.id FROM periods p1
  INNER JOIN periods p2 ON p1.building_id = p2.building_id
    AND p1.start_date = p2.start_date
    AND p1.num_days = p2.num_days
    AND p1.id < p2.id
);
DELETE FROM periods WHERE id IN (
  SELECT p1.id FROM periods p1
  INNER JOIN periods p2 ON p1.building_id = p2.building_id
    AND p1.start_date = p2.start_date
    AND p1.num_days = p2.num_days
    AND p1.id < p2.id
);

-- Make building_id NOT NULL
CREATE TABLE periods_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name TEXT NOT NULL,
  start_date TEXT NOT NULL,
  num_days INTEGER NOT NULL,
  is_current INTEGER NOT NULL DEFAULT 0,
  building_id INTEGER NOT NULL REFERENCES buildings(id)
);
INSERT INTO periods_new SELECT id, name, start_date, num_days, is_current, building_id FROM periods;
DROP TABLE periods;
ALTER TABLE periods_new RENAME TO periods;
