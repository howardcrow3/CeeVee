-- Network builder: meetings planned with a contact. Finishing one writes it to their timeline.
CREATE TABLE meeting (
  id INTEGER PRIMARY KEY,
  contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  starts_at TEXT NOT NULL,            -- ISO-8601 UTC
  minutes INTEGER NOT NULL DEFAULT 30,
  done INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX meeting_start ON meeting(done, starts_at);
