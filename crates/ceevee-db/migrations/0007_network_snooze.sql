-- Keep-in-touch reminders: a person can be put off until a date.
ALTER TABLE contact ADD COLUMN snoozed_until TEXT;
