-- What was prepared or sent for a job, kept so the person can open exactly what went out.
-- Resume and letter are referenced by document_version id: versions are never edited, so the exact
-- text can always be read back. The posting text is copied because postings disappear.
CREATE TABLE application_record (
  id INTEGER PRIMARY KEY,
  posting_id INTEGER REFERENCES posting(id) ON DELETE SET NULL,
  application_id INTEGER REFERENCES application(id) ON DELETE SET NULL,
  event TEXT NOT NULL CHECK (event IN ('prepared','submitted')),
  url TEXT NOT NULL,
  ats TEXT,
  job_title TEXT,
  posting_text TEXT,
  resume_version_id INTEGER REFERENCES document_version(id) ON DELETE SET NULL,
  cover_letter_version_id INTEGER REFERENCES document_version(id) ON DELETE SET NULL,
  filled TEXT NOT NULL DEFAULT '[]',   -- JSON: [{label, value, review}]
  skipped TEXT NOT NULL DEFAULT '[]',  -- JSON: [{label, required, reason}]
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX application_record_posting ON application_record(posting_id, created_at);
CREATE INDEX application_record_application ON application_record(application_id, created_at);
