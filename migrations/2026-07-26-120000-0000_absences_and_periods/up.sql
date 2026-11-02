-- New absences table with date ranges instead of period_id
CREATE TABLE absences_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    person_id INTEGER NOT NULL REFERENCES people(id),
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT 'absent'
);

-- Migrate old data: period_id -> date range from periods table
INSERT INTO absences_new (person_id, start_date, end_date, reason)
SELECT a.person_id, p.start_date, DATE(p.start_date, '+' || p.num_days || ' days'), a.reason
FROM absences a
JOIN periods p ON a.period_id = p.id;

-- Drop old absences
DROP TABLE absences;

-- Rename new table
ALTER TABLE absences_new RENAME TO absences;

CREATE INDEX idx_absences_person ON absences(person_id);
CREATE INDEX idx_absences_dates ON absences(start_date, end_date);
