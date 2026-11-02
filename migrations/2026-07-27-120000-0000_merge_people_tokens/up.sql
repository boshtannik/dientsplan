CREATE TABLE people_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    gender TEXT NOT NULL CHECK(gender IN ('M','F')),
    building_id INTEGER REFERENCES buildings(id),
    active INTEGER NOT NULL DEFAULT 1,
    token TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL DEFAULT 'resident' CHECK(role IN ('superadmin', 'admin', 'resident')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    used_at TEXT
);

INSERT INTO people_new (name, gender, building_id, active, token, role, created_at, used_at)
SELECT
    COALESCE(bt.used_by_name, 'Person-' || bt.id),
    COALESCE(bt.used_by_gender, 'M'),
    bt.building_id,
    bt.active,
    bt.token,
    bt.role,
    bt.created_at,
    bt.used_at
FROM building_tokens bt;

INSERT INTO people_new (name, gender, building_id, active, token, role, created_at, used_at)
SELECT
    p.name,
    p.gender,
    p.building_id,
    p.active,
    lower(hex(randomblob(16))),
    'resident',
    datetime('now'),
    NULL
FROM people p
WHERE NOT EXISTS (
    SELECT 1 FROM people_new pn
    WHERE pn.name = p.name AND pn.building_id = p.building_id
);

DROP TABLE building_tokens;
DROP TABLE people;

ALTER TABLE people_new RENAME TO people;

CREATE INDEX idx_people_building ON people(building_id);
CREATE INDEX idx_people_token ON people(token);
