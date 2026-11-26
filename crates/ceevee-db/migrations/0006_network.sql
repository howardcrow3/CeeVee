-- Network builder: how close a contact is, how it was met, a timeline of touches, and a record of
-- job changes seen when a newer LinkedIn export is imported. All of it stays on this machine.
ALTER TABLE contact ADD COLUMN tier TEXT NOT NULL DEFAULT 'warm' CHECK (tier IN ('close','warm','weak'));
ALTER TABLE contact ADD COLUMN how_met TEXT;
ALTER TABLE contact ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';           -- JSON array of strings
ALTER TABLE contact ADD COLUMN phone TEXT;
ALTER TABLE contact ADD COLUMN last_contacted TEXT;
ALTER TABLE contact ADD COLUMN created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'));
CREATE INDEX contact_tier ON contact(tier, last_contacted);

CREATE TABLE interaction (
  id INTEGER PRIMARY KEY,
  contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('note','email','meeting','call','message')),
  summary TEXT NOT NULL,
  at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX interaction_contact ON interaction(contact_id, at);

CREATE TABLE contact_change (
  id INTEGER PRIMARY KEY,
  contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE CASCADE,
  from_company TEXT, from_title TEXT,
  to_company TEXT, to_title TEXT,
  seen_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX contact_change_seen ON contact_change(seen_at);
