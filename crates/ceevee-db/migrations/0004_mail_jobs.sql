-- Jobs found in email: where an application came from, and whether the user has confirmed it ----
-- origin: 'manual' (added by hand or from a posting) or 'email' (CeeVee created it from a message).
-- review: an email-created job stays 'pending' until the user confirms it, or becomes 'dismissed'.
ALTER TABLE application ADD COLUMN origin TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE application ADD COLUMN review TEXT NOT NULL DEFAULT 'confirmed' CHECK (review IN ('confirmed','pending','dismissed'));
CREATE INDEX application_review ON application(review);
-- Mail that was read once and found not to be about a job search. Only the ID is kept, so a later
-- sync does not read it (or ask the model about it) again.
CREATE TABLE mail_seen (
  account_id INTEGER NOT NULL REFERENCES mail_account(id) ON DELETE CASCADE,
  message_id TEXT NOT NULL,
  PRIMARY KEY (account_id, message_id)
);
