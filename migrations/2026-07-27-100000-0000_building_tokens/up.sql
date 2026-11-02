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

CREATE INDEX idx_building_tokens_token ON building_tokens(token);
CREATE INDEX idx_building_tokens_building ON building_tokens(building_id);
