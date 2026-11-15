-- Mail tracking: connection settings, threading and reading state ------------
ALTER TABLE mail_account ADD COLUMN config TEXT NOT NULL DEFAULT '{}';   -- host, port, user; never a secret
ALTER TABLE mail_account ADD COLUMN sync_state TEXT NOT NULL DEFAULT '{}'; -- per provider cursor (IMAP UID, Gmail history id)
ALTER TABLE email_message ADD COLUMN to_addr TEXT;
ALTER TABLE email_message ADD COLUMN from_name TEXT;
ALTER TABLE email_message ADD COLUMN refs TEXT NOT NULL DEFAULT '[]';     -- Message-IDs this mail replies to, oldest first
ALTER TABLE email_message ADD COLUMN kind TEXT NOT NULL DEFAULT 'employer' CHECK (kind IN ('employer','alert'));
ALTER TABLE email_message ADD COLUMN needs_reply INTEGER NOT NULL DEFAULT 0;
ALTER TABLE email_message ADD COLUMN is_read INTEGER NOT NULL DEFAULT 0;
ALTER TABLE email_message ADD COLUMN draft_document_id INTEGER REFERENCES document(id) ON DELETE SET NULL;
CREATE INDEX email_message_received ON email_message(received_at);
