-- Revert: date ranges back to period-based absences
CREATE TABLE absences_old (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    person_id INTEGER NOT NULL REFERENCES people(id),
    period_id INTEGER NOT NULL REFERENCES periods(id),
    reason TEXT NOT NULL DEFAULT 'absent',
    UNIQUE(person_id, period_id)
);

-- Best-effort: pick the first period that overlaps each absence
INSERT INTO absences_old (person_id, period_id, reason)
SELECT a.person_id, p.id, a.reason
FROM absences a
JOIN periods p ON p.start_date <= a.end_date AND DATE(p.start_date, '+' || p.num_days || ' days') >= a.start_date
GROUP BY a.person_id, a.start_date;

DROP TABLE absences;
ALTER TABLE absences_old RENAME TO absences;

CREATE INDEX idx_absences_period ON absences(period_id);
CREATE INDEX idx_absences_person ON absences(person_id);
