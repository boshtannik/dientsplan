CREATE TABLE buildings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    building_id INTEGER NOT NULL REFERENCES buildings(id),
    key TEXT NOT NULL,
    name TEXT NOT NULL,
    required INTEGER NOT NULL DEFAULT 1,
    gender TEXT NOT NULL DEFAULT '' CHECK(gender IN ('M','F','')),
    active INTEGER NOT NULL DEFAULT 1,
    UNIQUE(building_id, key)
);

CREATE TABLE people (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    gender TEXT NOT NULL CHECK(gender IN ('M','F')),
    building_id INTEGER REFERENCES buildings(id),
    active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE periods (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    start_date TEXT NOT NULL,
    num_days INTEGER NOT NULL DEFAULT 7,
    is_current INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    period_id INTEGER NOT NULL REFERENCES periods(id),
    person_id INTEGER NOT NULL REFERENCES people(id),
    room_id INTEGER NOT NULL REFERENCES rooms(id),
    day INTEGER NOT NULL CHECK(day >= 0),
    UNIQUE(period_id, room_id, day, person_id)
);

CREATE TABLE absences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    person_id INTEGER NOT NULL REFERENCES people(id),
    period_id INTEGER NOT NULL REFERENCES periods(id),
    reason TEXT NOT NULL DEFAULT 'absent',
    UNIQUE(person_id, period_id)
);

CREATE INDEX idx_assignments_period ON assignments(period_id);
CREATE INDEX idx_assignments_person ON assignments(person_id);
CREATE INDEX idx_rooms_building ON rooms(building_id);
CREATE INDEX idx_people_building ON people(building_id);
