CREATE TABLE people_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    gender TEXT NOT NULL CHECK(gender IN ('M','F')),
    building_id INTEGER REFERENCES buildings(id),
    active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE building_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    building_id INTEGER NOT NULL REFERENCES buildings(id),
    token TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL DEFAULT 'admin' CHECK(role IN ('superadmin', 'admin', 'resident')),
    used_by_name TEXT,
    used_by_gender TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    used_at TEXT,
    active INTEGER NOT NULL DEFAULT 1
);

INSERT INTO people_old (id, name, gender, building_id, active)
SELECT id, name, gender, building_id, active FROM people;

INSERT INTO building_tokens (building_id, token, role, used_by_name, used_by_gender, created_at, used_at, active)
SELECT building_id, token, role, name, gender, created_at, used_at, active FROM people;

DROP TABLE people;
ALTER TABLE people_old RENAME TO people;

CREATE INDEX idx_people_building ON people(building_id);
CREATE INDEX idx_building_tokens_token ON building_tokens(token);
CREATE INDEX idx_building_tokens_building ON building_tokens(building_id);
