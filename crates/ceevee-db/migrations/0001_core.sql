-- CeeVee core schema. Timestamps are ISO-8601 UTC text. JSON columns hold AI
-- output next to the prompt version that produced it (`*_with`).

-- Profile ------------------------------------------------------------------
CREATE TABLE profile (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  full_name TEXT NOT NULL DEFAULT '',
  email TEXT, phone TEXT, location TEXT, summary TEXT,
  links TEXT NOT NULL DEFAULT '[]',
  photo_path TEXT,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE profile_entry (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('experience','education','project','certification','publication','reference')),
  title TEXT NOT NULL, organization TEXT, location TEXT,
  start_date TEXT, end_date TEXT, bullets TEXT NOT NULL DEFAULT '[]',
  sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE skill (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, skill_group TEXT, sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE language (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, level TEXT
);
CREATE TABLE fact (
  id INTEGER PRIMARY KEY,
  question_key TEXT NOT NULL UNIQUE,
  answer TEXT NOT NULL,
  sensitive INTEGER NOT NULL DEFAULT 0 CHECK (sensitive IN (0,1)), -- EEO, salary: never filled without asking
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Jobs ---------------------------------------------------------------------
CREATE TABLE document (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('base_resume','resume_variant','tailored_resume','cover_letter','email')),
  title TEXT NOT NULL,
  application_id INTEGER, category_id INTEGER,
  current_version_id INTEGER,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE job_category (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  criteria TEXT NOT NULL DEFAULT '{}', -- industries, functions, level range, titles, locations, pay, keywords, required/preferred
  priority INTEGER NOT NULL DEFAULT 0,
  active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1)),
  resume_document_id INTEGER REFERENCES document(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE company (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL, aliases TEXT NOT NULL DEFAULT '[]',
  website TEXT, email_domains TEXT NOT NULL DEFAULT '[]',
  careers_url TEXT, ats TEXT, industry TEXT, size TEXT, location TEXT,
  status TEXT NOT NULL DEFAULT 'normal' CHECK (status IN ('normal','watchlist','blocked')),
  notes TEXT
);
CREATE INDEX company_name ON company(name);
CREATE TABLE posting (
  id INTEGER PRIMARY KEY,
  source TEXT NOT NULL, url TEXT NOT NULL UNIQUE,
  company_id INTEGER REFERENCES company(id) ON DELETE SET NULL,
  title TEXT NOT NULL, location TEXT, level TEXT, pay TEXT, body TEXT,
  extracted TEXT, extracted_with TEXT,
  score REAL, state TEXT NOT NULL DEFAULT 'new' CHECK (state IN ('new','saved','dismissed')),
  first_seen TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  last_seen TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX posting_company ON posting(company_id);
CREATE TABLE posting_category (
  posting_id INTEGER NOT NULL REFERENCES posting(id) ON DELETE CASCADE,
  category_id INTEGER NOT NULL REFERENCES job_category(id) ON DELETE CASCADE,
  score REAL NOT NULL, score_reasons TEXT, scored_with TEXT,
  PRIMARY KEY (posting_id, category_id)
);
CREATE INDEX posting_category_by_category ON posting_category(category_id, score);
CREATE INDEX posting_category_by_posting ON posting_category(posting_id);
CREATE TABLE application (
  id INTEGER PRIMARY KEY,
  posting_id INTEGER REFERENCES posting(id) ON DELETE SET NULL,
  company_id INTEGER REFERENCES company(id) ON DELETE SET NULL,
  category_id INTEGER REFERENCES job_category(id) ON DELETE SET NULL,
  status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft','sent','screening','interview','offer','accepted','rejected','declined','withdrawn')),
  extracted TEXT, extracted_with TEXT, match TEXT, match_with TEXT,
  tone TEXT, language TEXT NOT NULL DEFAULT 'en-US',
  applied_at TEXT, next_interview_at TEXT, board_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX application_company ON application(company_id);
CREATE INDEX application_category_status ON application(category_id, status);
CREATE TABLE status_event (
  id INTEGER PRIMARY KEY,
  application_id INTEGER NOT NULL REFERENCES application(id) ON DELETE CASCADE,
  from_status TEXT, to_status TEXT NOT NULL,
  cause TEXT NOT NULL CHECK (cause IN ('manual','email','autofill','agent')),
  at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX status_event_application ON status_event(application_id);

-- People -------------------------------------------------------------------
CREATE TABLE contact (
  id INTEGER PRIMARY KEY, full_name TEXT NOT NULL, email TEXT, profile_url TEXT,
  source TEXT NOT NULL DEFAULT 'manual', notes TEXT
);
CREATE TABLE contact_role (
  id INTEGER PRIMARY KEY,
  contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE CASCADE,
  company_id INTEGER NOT NULL REFERENCES company(id) ON DELETE CASCADE,
  title TEXT, is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0,1)),
  start_date TEXT, end_date TEXT
);
CREATE INDEX contact_role_company ON contact_role(company_id, is_current);
CREATE TABLE outreach (
  id INTEGER PRIMARY KEY,
  contact_id INTEGER NOT NULL REFERENCES contact(id) ON DELETE CASCADE,
  application_id INTEGER REFERENCES application(id) ON DELETE SET NULL,
  message TEXT, status TEXT NOT NULL DEFAULT 'asked' CHECK (status IN ('asked','replied','referred')),
  asked_at TEXT, replied_at TEXT
);

-- Documents ----------------------------------------------------------------
CREATE TABLE document_version (
  id INTEGER PRIMARY KEY,
  document_id INTEGER NOT NULL REFERENCES document(id) ON DELETE CASCADE,
  content TEXT NOT NULL, -- resume JSON or letter Markdown
  cause TEXT NOT NULL, created_with TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX document_version_document ON document_version(document_id);
CREATE TABLE chat_message (
  id INTEGER PRIMARY KEY,
  application_id INTEGER REFERENCES application(id) ON DELETE CASCADE,
  document_id INTEGER REFERENCES document(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK (role IN ('user','assistant','tool')), content TEXT NOT NULL,
  at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE note (
  id INTEGER PRIMARY KEY,
  application_id INTEGER REFERENCES application(id) ON DELETE CASCADE,
  company_id INTEGER REFERENCES company(id) ON DELETE CASCADE,
  body TEXT NOT NULL,
  at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE theme (
  id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, builtin INTEGER NOT NULL DEFAULT 0, definition TEXT NOT NULL DEFAULT '{}'
);

-- Automation ---------------------------------------------------------------
CREATE TABLE apply_run (
  id INTEGER PRIMARY KEY,
  application_id INTEGER NOT NULL REFERENCES application(id) ON DELETE CASCADE,
  ats TEXT, outcome TEXT, started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), finished_at TEXT
);
CREATE TABLE apply_field (
  id INTEGER PRIMARY KEY,
  apply_run_id INTEGER NOT NULL REFERENCES apply_run(id) ON DELETE CASCADE,
  label TEXT NOT NULL, value TEXT, source_fact_id INTEGER REFERENCES fact(id) ON DELETE SET NULL, confidence REAL
);

-- Mail ---------------------------------------------------------------------
CREATE TABLE mail_account (
  id INTEGER PRIMARY KEY, provider TEXT NOT NULL CHECK (provider IN ('gmail','outlook','imap')),
  address TEXT NOT NULL UNIQUE, secret_ref TEXT, -- keychain reference only; never the secret
  last_synced_at TEXT
);
CREATE TABLE email_message (
  id INTEGER PRIMARY KEY,
  account_id INTEGER NOT NULL REFERENCES mail_account(id) ON DELETE CASCADE,
  message_id TEXT NOT NULL, thread_id TEXT, direction TEXT NOT NULL CHECK (direction IN ('in','out')),
  application_id INTEGER REFERENCES application(id) ON DELETE SET NULL,
  from_addr TEXT, subject TEXT, body TEXT, received_at TEXT,
  label TEXT, label_with TEXT, summary TEXT, suggested_status TEXT, interview_at TEXT,
  UNIQUE (account_id, message_id)
);
CREATE INDEX email_message_application ON email_message(application_id);
CREATE INDEX email_message_thread ON email_message(thread_id);

-- Google (optional) --------------------------------------------------------
CREATE TABLE google_account (
  id INTEGER PRIMARY KEY, address TEXT NOT NULL UNIQUE, scopes TEXT NOT NULL DEFAULT '[]', secret_ref TEXT
);
CREATE TABLE drive_file (
  id INTEGER PRIMARY KEY,
  document_version_id INTEGER NOT NULL REFERENCES document_version(id) ON DELETE CASCADE,
  drive_doc_id TEXT, drive_pdf_id TEXT
);
CREATE TABLE drive_sync (
  id INTEGER PRIMARY KEY,
  document_version_id INTEGER NOT NULL REFERENCES document_version(id) ON DELETE CASCADE,
  state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','done','failed')),
  attempts INTEGER NOT NULL DEFAULT 0, last_error TEXT
);

-- Agents -------------------------------------------------------------------
CREATE TABLE policy (
  id INTEGER PRIMARY KEY,
  category_id INTEGER REFERENCES job_category(id) ON DELETE CASCADE, -- NULL = global
  level TEXT NOT NULL DEFAULT 'assisted' CHECK (level IN ('manual','assisted','review_before_submit','autopilot','full_automatic')),
  action_modes TEXT NOT NULL DEFAULT '{}', -- per action: off | ask | auto
  daily_caps TEXT NOT NULL DEFAULT '{}',
  UNIQUE (category_id)
);
CREATE TABLE policy_ledger (
  id INTEGER PRIMARY KEY, day TEXT NOT NULL, action TEXT NOT NULL, category_id INTEGER, count INTEGER NOT NULL DEFAULT 0,
  UNIQUE (day, action, category_id)
);
CREATE TABLE consent (
  id INTEGER PRIMARY KEY, kind TEXT NOT NULL, scope TEXT, granted_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), revoked_at TEXT
);
CREATE TABLE agent_run (
  id INTEGER PRIMARY KEY, agent TEXT NOT NULL, trigger TEXT, state TEXT NOT NULL DEFAULT 'running',
  started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), finished_at TEXT
);
CREATE TABLE agent_step (
  id INTEGER PRIMARY KEY, run_id INTEGER NOT NULL REFERENCES agent_run(id) ON DELETE CASCADE,
  tool TEXT, input TEXT, output TEXT, risk TEXT, at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE INDEX agent_step_run ON agent_step(run_id);
CREATE TABLE agent_memory (
  id INTEGER PRIMARY KEY, kind TEXT NOT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
CREATE TABLE proposal (
  id INTEGER PRIMARY KEY, run_id INTEGER REFERENCES agent_run(id) ON DELETE SET NULL,
  kind TEXT NOT NULL, payload TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','approved','rejected','expired')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), decided_at TEXT
);
CREATE TABLE chat_summary (
  id INTEGER PRIMARY KEY, scope TEXT NOT NULL, summary TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- System -------------------------------------------------------------------
CREATE TABLE job (
  id INTEGER PRIMARY KEY, kind TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'queued' CHECK (state IN ('queued','running','done','failed','cancelled')),
  steps TEXT NOT NULL DEFAULT '[]', progress REAL NOT NULL DEFAULT 0, error TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')), updated_at TEXT
);
CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE license_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  trial_started_at TEXT, license_blob TEXT, last_checked_at TEXT
);
