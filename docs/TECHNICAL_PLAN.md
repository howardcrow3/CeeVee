# CeeVee: Technical Plan

Last updated 2026-11-10. Companion to [PLAN.md](PLAN.md), which holds the product scope and decisions. This file says how CeeVee is built: repository layout, components, data flow, the open source projects it includes, and the engineering order of work.

## 1. Ground rules

- **Clean room.** Dossier (AGPL-3.0) is a product reference only. No code, prompts, CSS, images or schema text is copied from it.
- **Licenses we accept** in anything that ships: MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode, MPL-2.0 (file-level, unmodified), public domain. **Rejected:** GPL, LGPL (static linking), AGPL, SSPL, and model licenses that restrict commercial use. `cargo-deny` and `license-checker` enforce this in CI.
- **Local first.** Profile, documents, mail and history never leave the machine. The only network calls are job-source queries (search terms only), mail sync, model download, license activation and update checks.
- **Human in the loop.** Nothing is submitted, sent or saved to the profile without an explicit user action.
- **Targets.** Windows 11 x64 and macOS 13+ on Apple silicon (arm64). No Intel Mac or Linux build in v1.

## 2. Repository layout

One repository, a Cargo workspace plus a SvelteKit app.

```
CeeVee/
├─ apps/
│  └─ desktop/                # Tauri 2 app
│     ├─ src-tauri/           # Rust entry point, commands, events, plugins, capabilities
│     └─ ui/                  # SvelteKit (static adapter), TypeScript, Tailwind
├─ crates/
│  ├─ ceevee-core/            # domain types, errors, job runner, event bus
│  ├─ ceevee-db/              # SQLite schema, migrations, repositories, FTS
│  ├─ ceevee-ai/              # llama.cpp runtime, model manager, tasks, grammars, prompts
│  ├─ ceevee-profile/         # resume and LinkedIn import, merge, fact extraction from chat
│  ├─ ceevee-docs/            # resume and cover letter rendering (Typst), DOCX export, themes
│  ├─ ceevee-search/          # categories, job sources, dedupe, scoring, scheduler
│  ├─ ceevee-apply/           # browser automation, ATS adapters, generic form filler
│  ├─ ceevee-mail/            # IMAP/SMTP, OAuth, classification, linking, alert-email parsing
│  ├─ ceevee-network/         # contacts, LinkedIn Connections import, company matching
│  └─ ceevee-license/         # trial, activation, Ed25519 license verification
├─ adapters/                  # versioned ATS adapter definitions (data, updatable without a release)
├─ themes/                    # Typst resume and cover letter templates
├─ prompts/                   # versioned prompt files, one per AI task
├─ tests/                     # fixtures: postings, resumes, ATS form snapshots, emails
└─ docs/
```

Crates depend downward only: `desktop → feature crates → ceevee-db, ceevee-ai → ceevee-core`. Feature crates never call each other directly; they publish and subscribe through the core event bus, so each can be tested alone.

## 3. Runtime architecture

- **One process.** The Tauri app hosts the Rust core and one webview. Long work runs on a Tokio runtime inside the core.
- **UI ↔ core.** Typed Tauri commands for requests, Tauri events for progress and live updates. TypeScript bindings are generated from Rust types with `specta` and `tauri-specta`, so the UI and core cannot drift.
- **Job runner.** Every long task (draft documents, import resume, run searches, auto-fill, mail sync) is a `Job` row with named steps. The UI subscribes by job ID and can reattach after a reload. Jobs are resumable where it matters (search, mail sync) and cancellable.
- **Scheduler.** A single in-process scheduler runs category searches daily and mail sync every 10 minutes while the app or tray icon is running. On macOS the app can keep running from the menu bar; on Windows from the system tray. Optional launch at login.
- **Model worker.** One llama.cpp context owned by a dedicated thread with a priority queue: interactive tasks (chat, drafting) jump ahead of background tasks (scoring, mail labels). The model is unloaded after 10 idle minutes to free memory.
- **Automation window.** A separate Chrome or Edge process with its own user-data folder, driven over CDP. It is visible to the user at all times while filling.

## 4. Data layer

- SQLite in WAL mode, one file in the OS app-data folder (`%APPDATA%\TurtleBox\CeeVee` on Windows, `~/Library/Application Support/studio.turtlebox.ceevee` on macOS).
- `sqlx` with compile-time checked queries and embedded, numbered migrations. A backup copy is written before every migration.
- FTS5 virtual tables over postings, applications, emails and notes for global search.
- JSON columns for AI output (`extracted`, `match`, `score_reasons`, `email_label`) with a `*_with` column holding the prompt version that produced it.
- Tables as listed in PLAN.md (20 core tables). Indexes: `posting(company_id)`, `posting_category(category_id, score)`, `posting_category(posting_id)`, `application(company_id)`, `application(category_id, status)`, `contact_role(company_id, is_current)`, `email_message(application_id)`, `email_message(thread_id)`.
- Secrets (OAuth refresh tokens, IMAP app passwords, optional API keys) live in the OS keychain only. The database stores a reference.
- Export and import: one JSON file with a schema version, plus a zip of rendered documents. Nightly local backup with 14-day retention.

## 5. AI layer

**Runtime.** llama.cpp linked through `llama-cpp-2`. Metal on Apple silicon; Vulkan on Windows with CPU fallback. CUDA is not shipped in v1 (installer size), though the Vulkan path runs on NVIDIA cards.

**Models** (downloaded on first run from the Turtlebox CDN, SHA-256 verified, resumable):

| Use | Default | Size (Q4_K_M GGUF) | License | When |
| --- | --- | --- | --- | --- |
| General (drafting, extraction, chat) | Qwen3 4B Instruct | about 2.5 GB | Apache-2.0 | 8 to 15 GB RAM |
| General, larger | Qwen3 8B Instruct | about 5 GB | Apache-2.0 | 16 GB RAM and up, offered |
| Embeddings (matching, dedupe, company names) | bge-small-en-v1.5 (GGUF) | about 70 MB | MIT | Always |

Model choice is a config file, so a better small model can be swapped in with an update. Llama-family and Gemma models are avoided because their licenses add terms for redistribution.

**Tasks.** Each AI task is a Rust module with a versioned prompt file in `prompts/`, a JSON schema for its output and a GBNF grammar compiled from that schema, so output always parses. v1 tasks:

| Task | Input | Output |
| --- | --- | --- |
| `import-resume` | Resume text | Profile entries |
| `merge-profile` | Two profile drafts | Merged entries plus conflicts |
| `extract-posting` | Posting text | Title, company, level, location, pay, requirements |
| `match-posting` | Profile, requirements | Met, partly met, gaps, with evidence |
| `score-posting` | Profile, category, posting | 0 to 100, reasons, red flags |
| `tailor-resume` | Base resume, posting, match | Section edits (reorder, reword, hide) |
| `write-cover-letter` | Profile, posting, tone | Letter parts |
| `refine` | Document, chat message | Document edits |
| `detect-facts` | Chat message, profile | Proposed profile changes |
| `map-form-fields` | Form field list, profile facts | Field to value with confidence |
| `answer-question` | Free-text form question, profile, posting | Answer draft |
| `label-email` | Email, candidate applications | Kind, linked application, suggested status, interview time, questions |
| `draft-reply` | Email thread, profile | Reply draft |
| `referral-message` | Contact, posting, profile | Message draft |

**Fact guard.** After any drafting task, code checks that every employer, title, date, number and skill in the output exists in the profile or the posting. Anything else is flagged and the draft is not saved until the user resolves it.

**Optional providers.** Ollama, LM Studio, any OpenAI-compatible endpoint, and Anthropic, behind one `LlmProvider` trait. Keys in the keychain.

## 6. Profile import and learning

- **Resume:** `pdf-extract` / `lopdf` for PDF text, `docx-rs` for Word, plain read for Markdown. Text goes to `import-resume`.
- **LinkedIn profile PDF:** same PDF path with a LinkedIn-specific section splitter before the model.
- **LinkedIn data export:** unzip with `zip`, parse `Positions.csv`, `Education.csv`, `Skills.csv`, `Certifications.csv`, `Projects.csv`, `Languages.csv` and `Connections.csv` with `csv`. No AI needed for these.
- **Merge:** entries matched on normalized company name (embedding similarity plus aliases), title and overlapping dates; conflicts go to a review screen.
- **Learning from chat:** `detect-facts` runs after each user chat turn; proposals render as cards; accepted ones write to the profile with a `source_chat_id`.

## 7. Documents

- Resumes and cover letters are rendered with **Typst** embedded as a library: one Typst template per theme in `themes/`, fed JSON from the profile or application copy. Output is a vector PDF with selectable text, which ATS parsers read reliably. US Letter by default, A4 optional.
- Live preview in the UI renders the same Typst output to SVG pages.
- DOCX export through `docx-rs` from the same data, for employers that ask for Word.
- Document versions store the structured data, not the PDF; PDFs are re-rendered on demand and cached.

## 8. Job search

- **Category → query builder.** Each active category produces a query per source: titles plus synonyms, location and radius or remote, level words, include and exclude keywords.
- **Source trait.** `JobSource { search(query) -> Vec<RawPosting> }` with implementations:
  - Company career sites: Greenhouse Job Board API, Lever Postings API, Ashby Job Board API, SmartRecruiters Posting API, Workable, Recruitee, Workday career-site JSON feed, and a generic schema.org `JobPosting` reader (JSON-LD and microdata) for any careers page. Employers to poll come from the watchlist and from companies seen in results.
  - Free boards: USAJOBS API, Adzuna API (free tier), Remotive API, RSS readers for We Work Remotely and similar.
  - Alert emails: parsers for Indeed, LinkedIn, ZipRecruiter and Glassdoor job alert emails, fed by the mail pipeline.
  - Optional paid: JSearch, with the user's own key.
- **Normalize.** Every result maps to one `Posting` shape; companies are matched or created (domain, then normalized name, then embedding similarity).
- **Dedupe.** Canonical URL (tracking parameters stripped), then company plus normalized title plus location, then embedding similarity above a threshold. Company-site links win over board links.
- **Score.** Hard filters from required category fields first (cheap, no model), then `score-posting` on what remains, plus a bonus when the user knows someone at the company.
- **Politeness.** Per-host rate limits, `robots.txt` honored for HTML pages, back-off on errors, 30-minute response cache.

## 9. Auto-fill engine

- **Browser.** `chromiumoxide` drives Microsoft Edge (present on every Windows 11 PC) or Google Chrome; if neither is found, Chrome for Testing is downloaded. A dedicated user-data folder keeps the user's logins to job sites between runs.
- **ATS detection.** URL patterns and page markers identify Workday, Greenhouse, Lever, iCIMS, Ashby and the v1.x systems.
- **Adapters.** Declarative files in `adapters/` (selectors, field meanings, step order, upload targets), versioned and downloadable as updates so a site change does not need a new app release. A small Rust trait covers the steps declarative files cannot express (Workday's multi-page flow, iCIMS iframes).
- **Generic path.** For unknown forms: read labels, input types, options and the accessibility tree through CDP; `map-form-fields` maps each field to a profile fact with a confidence; fields below the threshold are left empty and highlighted.
- **Uploads.** Tailored resume and cover letter PDFs uploaded through CDP file-chooser interception.
- **Sensitive answers.** EEO, veteran, disability, salary and sponsorship questions are filled only from answers the user saved for that purpose, and are always shown in the review list.
- **Stop before submit.** The engine never clicks a final submit control. It shows a checklist overlay and waits. Submission is detected by URL change and confirmation-text patterns, and the application moves to Sent with its documents frozen.
- **Logging.** Every run writes `apply_run` and `apply_field` rows; with the user's consent, anonymized field-mapping failures can be sent to Turtlebox to improve adapters.

## 10. Mail

- **Accounts.** Gmail and Microsoft 365/Outlook through OAuth 2 (PKCE, loopback redirect on a random local port, `oauth2` crate); any other provider through IMAP with an app password.
- **Gmail scope note.** Full IMAP access to Gmail is a restricted scope requiring Google's yearly security assessment. Until that is budgeted, Gmail users connect with an app password over IMAP, which needs no verification. Microsoft OAuth needs only publisher verification.
- **Sync.** `async-imap` with IDLE where supported, otherwise polling every 10 minutes; first sync covers 90 days of headers. Bodies are fetched only for candidate messages. `mail-parser` decodes MIME.
- **Candidate filter** (no model): sender or reply-to domain matches a company's domains, known ATS sender domains, replies in threads the user started from CeeVee, and known job-alert senders.
- **Labelling.** `label-email` assigns kind, application, suggested status and interview time. Alert emails go to the job-alert parsers instead.
- **Sending.** `lettre` over SMTP with the same credentials; every send is a user click.
- **Calendar.** Interview times from email produce `.ics` files with the `ics` crate; calendar sync is v2.

## 11. Network (connections)

- `Connections.csv` from the LinkedIn export becomes `contact` and current `contact_role` rows.
- Former-colleague matching compares contacts' companies with the user's own experience entries and dates.
- Email correspondents at company domains become contacts with source `email`.
- A "Save this profile" action in the automation browser captures one LinkedIn profile page the user opened, parsed into `contact_role` history. Never automatic or in bulk.
- Company matching reuses the search crate's company resolver.

## 12. Licensing client

- Trial start stored in the database and the keychain; the later of the two wins. A signed trial token is fetched from turtlebox.studio once, bound to a hashed machine ID (`machine-uid` hashed with SHA-256 and an app salt).
- License activation sends key and machine hash, receives an Ed25519-signed license file; `ed25519-dalek` verifies it offline against a public key compiled into the app.
- Clock rollback check: refuse to move the trial backwards past the newest timestamp seen in the database.
- After the trial without a license, the app is read-only: view and export only.

## 13. Server side (highturtle repo, AWS)

Small and serverless:

| Piece | AWS service | Purpose |
| --- | --- | --- |
| License fulfillment webhook | API Gateway + Lambda | FastSpring calls it on purchase; creates the key, stores the order |
| Activation API | API Gateway + Lambda + DynamoDB | Activate, deactivate (free a seat), validate; 2 seats per license |
| Trial token API | Same | Issues signed trial tokens per machine hash |
| Signing key | AWS KMS or Secrets Manager | Holds the Ed25519 private key |
| Downloads and models | S3 + CloudFront | Installers, update manifest, GGUF models, adapter updates |
| Update manifest | S3 + CloudFront | Signed JSON read by the Tauri updater |

FastSpring is the merchant of record (sales tax, receipts, refunds).

## 14. Security and privacy

- Tauri capabilities restrict the webview to the commands it needs; strict Content Security Policy; no remote content in the main webview.
- Outbound HTTP allow-list per feature (job sources, mail servers, turtlebox.studio, model CDN).
- Automation browser runs with a separate profile and never shares cookies with the main webview.
- Crash reports and anonymous usage stats are opt-in.
- Data deletion: "Delete everything" removes the database, documents, keychain entries and the automation browser profile.

## 15. Testing

- **Rust:** unit tests per crate, `insta` snapshot tests for prompt rendering and parsers, SQLite tests against a temp database.
- **AI:** a mock provider for unit tests; an opt-in live suite that runs every task against the real default model on a fixed set of resumes and postings and checks schema validity and fact-guard results.
- **Job sources:** recorded API responses as fixtures; opt-in live tests per source.
- **Auto-fill:** saved HTML snapshots of each ATS form served from a local test server, filled by the real engine in headless Chrome; plus a monthly manual run against live employer sites.
- **Mail:** a local IMAP test server (GreenMail in Docker) with fixture mailboxes, including real-shape alert emails.
- **UI:** Vitest for components; Playwright against the SvelteKit dev build with a mocked command layer for end-to-end screens.

## 16. Build, CI and release

- GitHub Actions matrix: `windows-latest` (x64) and `macos-14` (arm64).
- Per pull request: `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, `cargo deny check`, `pnpm lint`, `pnpm test`, `svelte-check`.
- Release: `tauri-action` builds MSI/NSIS and DMG; Windows signed with Azure Trusted Signing; macOS signed with Developer ID and notarized; artifacts and the signed update manifest uploaded to S3.
- Version scheme: semantic versions; licenses cover the purchased major version.

## 17. Open source projects included

All are permissively licensed. Versions are pinned in `Cargo.lock` and `pnpm-lock.yaml`.

### Rust (core)

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| Tauri 2 | [tauri-apps/tauri](https://github.com/tauri-apps/tauri) | MIT / Apache-2.0 | App shell, IPC, installers |
| Tauri plugins (updater, single-instance, autostart, dialog, fs, log, notification, store, os) | [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) | MIT / Apache-2.0 | Updates, tray, file dialogs, notifications |
| tauri-specta, specta | [specta-rs/tauri-specta](https://github.com/specta-rs/tauri-specta) | MIT | Typed TypeScript bindings for commands |
| Tokio | [tokio-rs/tokio](https://github.com/tokio-rs/tokio) | MIT | Async runtime |
| Serde, serde_json | [serde-rs/serde](https://github.com/serde-rs/serde) | MIT / Apache-2.0 | Serialization |
| thiserror, anyhow | [dtolnay/thiserror](https://github.com/dtolnay/thiserror) | MIT / Apache-2.0 | Errors |
| tracing | [tokio-rs/tracing](https://github.com/tokio-rs/tracing) | MIT | Logging |
| SQLx | [launchbadge/sqlx](https://github.com/launchbadge/sqlx) | MIT / Apache-2.0 | SQLite access and migrations |
| SQLite | [sqlite.org](https://sqlite.org) | Public domain | Database (bundled) |
| keyring | [hwchen/keyring-rs](https://github.com/hwchen/keyring-rs) | MIT / Apache-2.0 | OS keychain |
| reqwest | [seanmonstar/reqwest](https://github.com/seanmonstar/reqwest) | MIT / Apache-2.0 | HTTP client |
| scraper | [rust-scraper/scraper](https://github.com/rust-scraper/scraper) | ISC | HTML parsing, JobPosting extraction |
| url, chrono, uuid | rust-lang ecosystem | MIT / Apache-2.0 | Utilities |

### AI

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| llama.cpp | [ggml-org/llama.cpp](https://github.com/ggml-org/llama.cpp) | MIT | Local inference (Metal, Vulkan, CPU) |
| llama-cpp-2 | [utilityai/llama-cpp-rs](https://github.com/utilityai/llama-cpp-rs) | MIT / Apache-2.0 | Rust bindings to llama.cpp |
| Qwen3 4B and 8B Instruct | [QwenLM/Qwen3](https://github.com/QwenLM/Qwen3) | Apache-2.0 | Default models |
| bge-small-en-v1.5 | [FlagOpen/FlagEmbedding](https://github.com/FlagOpen/FlagEmbedding) | MIT | Embeddings |
| jsonschema | [Stranger6667/jsonschema](https://github.com/Stranger6667/jsonschema) | MIT | Validating AI output |

### Documents and import

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| Typst | [typst/typst](https://github.com/typst/typst) | Apache-2.0 | Resume and cover letter PDF and SVG rendering |
| docx-rs | [bokuweb/docx-rs](https://github.com/bokuweb/docx-rs) | MIT | DOCX export and Word import |
| pdf-extract | [jrmuizel/pdf-extract](https://github.com/jrmuizel/pdf-extract) | MIT | PDF text extraction |
| lopdf | [J-F-Liu/lopdf](https://github.com/J-F-Liu/lopdf) | MIT | PDF fallback parsing |
| csv | [BurntSushi/rust-csv](https://github.com/BurntSushi/rust-csv) | MIT / Unlicense | LinkedIn export files |
| zip | [zip-rs/zip2](https://github.com/zip-rs/zip2) | MIT | LinkedIn export archive, data export |
| Fonts: Inter, Source Serif 4, IBM Plex | Google Fonts / Adobe / IBM | SIL OFL 1.1 | Resume themes and UI |

### Browser automation

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| chromiumoxide | [mattsse/chromiumoxide](https://github.com/mattsse/chromiumoxide) | MIT / Apache-2.0 | Driving Edge or Chrome over CDP |
| Chrome for Testing | [GoogleChromeLabs/chrome-for-testing](https://github.com/GoogleChromeLabs/chrome-for-testing) | BSD-3 (Chromium) | Fallback browser download |

### Mail and calendar

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| async-imap | [chatmail/async-imap](https://github.com/chatmail/async-imap) | MIT / Apache-2.0 | Reading mail |
| mail-parser | [stalwartlabs/mail-parser](https://github.com/stalwartlabs/mail-parser) | MIT / Apache-2.0 | MIME parsing |
| lettre | [lettre/lettre](https://github.com/lettre/lettre) | MIT | Sending mail |
| oauth2 | [ramosbugs/oauth2-rs](https://github.com/ramosbugs/oauth2-rs) | MIT / Apache-2.0 | Gmail and Microsoft sign-in |
| ics | [hummingly/ics](https://github.com/hummingly/ics) | MIT / Apache-2.0 | Interview calendar files |

### Licensing

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| ed25519-dalek | [dalek-cryptography/curve25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek) | BSD-3 | License signature verification |
| machine-uid | [Hanaasagi/machine-uid](https://github.com/Hanaasagi/machine-uid) | MIT | Machine ID for seats |
| sha2 | [RustCrypto/hashes](https://github.com/RustCrypto/hashes) | MIT / Apache-2.0 | Hashing machine ID, model checksums |

### Frontend

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| Svelte 5 and SvelteKit | [sveltejs/svelte](https://github.com/sveltejs/svelte), [sveltejs/kit](https://github.com/sveltejs/kit) | MIT | UI framework |
| Tailwind CSS | [tailwindlabs/tailwindcss](https://github.com/tailwindlabs/tailwindcss) | MIT | Styling and design tokens |
| Bits UI | [huntabyte/bits-ui](https://github.com/huntabyte/bits-ui) | MIT | Accessible headless components |
| Lucide | [lucide-icons/lucide](https://github.com/lucide-icons/lucide) | ISC | Icons |
| LayerChart | [techniq/layerchart](https://github.com/techniq/layerchart) | MIT | Dashboard funnel and weekly charts |
| svelte-dnd-action | [isaacHagoel/svelte-dnd-action](https://github.com/isaacHagoel/svelte-dnd-action) | MIT | Application board drag and drop |
| TipTap | [ueberdosis/tiptap](https://github.com/ueberdosis/tiptap) | MIT | Editing letter and email text |
| jsdiff | [kpdecker/jsdiff](https://github.com/kpdecker/jsdiff) | BSD-3 | "Pull from base" diffs |

### Development and CI

| Project | Repository | License | Used for |
| --- | --- | --- | --- |
| cargo-deny | [EmbarkStudios/cargo-deny](https://github.com/EmbarkStudios/cargo-deny) | MIT / Apache-2.0 | License and advisory checks |
| insta | [mitsuhiko/insta](https://github.com/mitsuhiko/insta) | Apache-2.0 | Snapshot tests |
| Vitest | [vitest-dev/vitest](https://github.com/vitest-dev/vitest) | MIT | UI tests |
| Playwright | [microsoft/playwright](https://github.com/microsoft/playwright) | Apache-2.0 | UI end-to-end tests |
| GreenMail | [greenmail-mail-test/greenmail](https://github.com/greenmail-mail-test/greenmail) | Apache-2.0 | Test IMAP server (dev only) |
| tauri-action | [tauri-apps/tauri-action](https://github.com/tauri-apps/tauri-action) | MIT | Release builds |

### Not included

| Project | Why |
| --- | --- |
| Dossier | AGPL-3.0; reference only |
| Ollama | Not bundled; supported only as an optional external provider |
| Llama and Gemma model weights | Licenses add redistribution terms; Qwen (Apache-2.0) avoids them |
| Puppeteer, Selenium, Playwright at runtime | Node or Java runtimes; chromiumoxide keeps automation in Rust |

## 18. Engineering order of work

Matches the roadmap in PLAN.md (about 27 weeks).

1. **Foundation (weeks 1 to 3).** Workspace and crates skeleton; Tauri app with sidebar, themes and tray; `ceevee-db` with first migrations; job runner and event bus; tauri-specta bindings; CI with signing on both platforms; cargo-deny policy.
2. **Local AI (weeks 4 to 6).** Model manager (hardware check, download, checksum, resume); llama.cpp worker with priority queue; grammar-constrained output; prompt and task framework; first tasks (`extract-posting`, `import-resume`); benchmark on an 8 GB Windows laptop and an M1 MacBook Air.
3. **Profile and documents (weeks 7 to 11).** Onboarding with resume, LinkedIn PDF and export import, merge review; profile editor and answers store; Typst themes (4), PDF and DOCX export; new application, match, tailoring, cover letter; refine chat, versions, fact guard, `detect-facts`.
4. **Tracking and search (weeks 12 to 17).** Board, table, dashboard, reminders, export, import, backups. Categories and companies; company-site sources (Greenhouse, Lever, Ashby, SmartRecruiters, Workday feed, JobPosting reader), USAJOBS and Adzuna; dedupe, scoring, scheduler; LinkedIn Connections import and the "who you know" panel.
5. **Auto-fill (weeks 18 to 23).** Browser launcher and CDP layer; adapters for Greenhouse, Lever and Ashby, then Workday and iCIMS; generic form path; review overlay; submit detection; adapter update channel.
6. **Mail and launch (weeks 24 to 27).** IMAP accounts, Microsoft OAuth, Gmail app-password flow; candidate filter, labelling, linking; job-alert parsers for Indeed, LinkedIn, ZipRecruiter, Glassdoor; licensing client and highturtle endpoints; updater; turtlebox.studio pages; beta with 20 to 50 testers.

## 19. Technical risks

| Risk | Mitigation |
| --- | --- |
| Vulkan driver problems on some Windows laptops | Detect at first run, fall back to CPU, report in the Usage screen |
| 4B model quality on tailoring | Narrow tasks, grammars, fact guard, optional 8B or hosted model; measure with the live AI suite each release |
| Workday's per-employer accounts and multi-page flow | Treat as the user's step: CeeVee pauses for sign-in, then resumes filling |
| Job alert email formats change | Parsers are data-driven and shipped through the adapter update channel; fixtures from real alerts in tests |
| Typst template quality vs. HTML themes | Prototype two themes in week 1 to confirm the look before committing |
| Webview differences (WebView2 vs WKWebView) | UI tested on both in CI with screenshots; avoid cutting-edge CSS |
