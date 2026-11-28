-- Interview practice: each question the person answered, with the coach's feedback, kept with the application.
CREATE TABLE practice_note (
  id INTEGER PRIMARY KEY,
  application_id INTEGER NOT NULL REFERENCES application(id) ON DELETE CASCADE,
  question TEXT NOT NULL,
  answer TEXT NOT NULL,
  feedback TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX practice_note_app ON practice_note(application_id, id);
