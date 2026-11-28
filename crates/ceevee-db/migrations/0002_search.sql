-- Global search (FTS5, external content, kept in sync by triggers).
CREATE VIRTUAL TABLE posting_fts USING fts5(title, body, content='posting', content_rowid='id');
CREATE TRIGGER posting_ai AFTER INSERT ON posting BEGIN
  INSERT INTO posting_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
END;
CREATE TRIGGER posting_ad AFTER DELETE ON posting BEGIN
  INSERT INTO posting_fts(posting_fts, rowid, title, body) VALUES ('delete', old.id, old.title, old.body);
END;
CREATE TRIGGER posting_au AFTER UPDATE ON posting BEGIN
  INSERT INTO posting_fts(posting_fts, rowid, title, body) VALUES ('delete', old.id, old.title, old.body);
  INSERT INTO posting_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
END;

CREATE VIRTUAL TABLE email_fts USING fts5(subject, body, content='email_message', content_rowid='id');
CREATE TRIGGER email_ai AFTER INSERT ON email_message BEGIN
  INSERT INTO email_fts(rowid, subject, body) VALUES (new.id, new.subject, new.body);
END;
CREATE TRIGGER email_ad AFTER DELETE ON email_message BEGIN
  INSERT INTO email_fts(email_fts, rowid, subject, body) VALUES ('delete', old.id, old.subject, old.body);
END;
CREATE TRIGGER email_au AFTER UPDATE ON email_message BEGIN
  INSERT INTO email_fts(email_fts, rowid, subject, body) VALUES ('delete', old.id, old.subject, old.body);
  INSERT INTO email_fts(rowid, subject, body) VALUES (new.id, new.subject, new.body);
END;

CREATE VIRTUAL TABLE note_fts USING fts5(body, content='note', content_rowid='id');
CREATE TRIGGER note_ai AFTER INSERT ON note BEGIN
  INSERT INTO note_fts(rowid, body) VALUES (new.id, new.body);
END;
CREATE TRIGGER note_ad AFTER DELETE ON note BEGIN
  INSERT INTO note_fts(note_fts, rowid, body) VALUES ('delete', old.id, old.body);
END;
CREATE TRIGGER note_au AFTER UPDATE ON note BEGIN
  INSERT INTO note_fts(note_fts, rowid, body) VALUES ('delete', old.id, old.body);
  INSERT INTO note_fts(rowid, body) VALUES (new.id, new.body);
END;
