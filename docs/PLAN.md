# CeeVee: Product and Technical Plan

Last updated 2026-11-11. This file mirrors the [live plan document](https://claude.ai/artifact/WjnrwY1qasFHHXThPjtVrb) and is updated as decisions are made. Engineering detail is in [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md).

## Summary

CeeVee is a paid desktop app for Windows 11 and Apple silicon Macs that runs a person's whole job search on their own computer, with a local AI model that works the moment it is installed. It takes [Dossier](https://github.com/AienTech/dossier) as its reference for features and look, then adds what Dossier deliberately leaves out: it fills in application forms in a browser and watches the inbox for replies on its own.

The key decisions:

- **Rust core in a Tauri 2 shell.** One small native app per platform, with the UI drawn in the system webview (WebView2 on Windows, WKWebView on macOS).
- **Local model built in.** llama.cpp is linked into the app, uses Metal on Macs and Vulkan on Windows, and a default model downloads on first run. No account, key or Ollama install needed. Hosted or Ollama models stay optional.
- **One SQLite file** holds the profile, jobs, applications, documents, emails and history.
- **Browser automation** fills applications in a separate Chrome or Edge window and stops before Submit, so the person always presses the final button.
- **Email tracking** over IMAP (Gmail and Outlook by OAuth, others by app password) links replies to applications and moves their status.
- **14-day full trial, then a license key** bought on turtlebox.studio, checked offline, with updates served from the same site.

Dossier is licensed AGPL-3.0, so CeeVee studies its features and design but writes every line of its own code. Nothing from its source, prompts, styles or assets is copied.

## Product scope

Version 1 matches Dossier's core loop (profile, tailored documents, tracking, inbox) and adds auto-fill and inbox watching. Dossier's server features (Docker, passwords, MCP connector, multi-database) are dropped because CeeVee is a single-user desktop app.

| Area | What Dossier does | What CeeVee does | Release |
| --- | --- | --- | --- |
| Profile | One career profile (experience, skills, projects, education, certifications, publications, references) with a strength meter; import from PDF, Word or Markdown | Same, plus a "facts" store of answers to common form questions (work authorization, notice period, salary range, EEO answers, links) for auto-fill | v1 |
| Base resume | 12 themes and a theme builder, printed to PDF | 4 to 6 themes at launch, PDF and DOCX export; theme builder later | v1, builder v1.x |
| New application | Paste a posting; AI extracts requirements, shows match and gaps, drafts tailored resume and cover letter | Same, also from a URL or a browser capture; never invents experience, gaps shown as gaps | v1 |
| Refine | Chat to edit, regenerate parts, version history, undo, notes | Same | v1 |
| Find jobs | Search JSearch, Adzuna, FreeHire; save, dismiss, score | Category-driven search of company career sites, job boards and job posting services (see Job sources), plus paste or capture any posting | v1 |
| Job agent | Up to 3 saved searches run daily, scored, optional drafts for review | Same idea, runs while the app is open or in the tray | v2 |
| Apply | Builds an email with PDFs; never applies | **Auto-fill**: opens the application page, fills every field from the profile and facts, uploads the tailored PDFs, writes answers to free-text questions, then pauses for review before Submit | v1 |
| Inbox | Gmail replies linked to applications, AI summary, drafted answers | **IMAP for any provider** (Gmail and Outlook by OAuth); classifies each reply (rejection, interview, assessment, offer, recruiter), links it, suggests a status change, drafts a reply | v1 |
| Calendar | Google Calendar sync, interviews on the dashboard | Interviews from emails shown on the dashboard, .ics export; Google and Outlook calendar sync later | v1, sync v2 |
| Track | Board (Draft to Offer), table, dashboard funnel, weekly chart, "worth a look next" | Same, with follow-up reminders when an application goes quiet for N days | v1 |
| Your data | Export, import, nightly backups, delete everything | Same, all local; backups to a folder the person picks | v1 |
| Usage | Tokens per AI task | Local model status, speed and disk use | v1 |

Navigation follows Dossier's left sidebar: Dashboard; You (Profile, Base resume, Answers); Job search (Find jobs, Agent); Applications; Inbox; Settings.

**Design direction.** Dossier's look is a warm, quiet one: a beige shell around a lighter panel, near-black primary buttons, thin hairline borders, one sans-serif face, light and dark modes that both meet WCAG AA contrast, and documents previewed on a canvas like paper. CeeVee adopts that direction (calm, document-first, few colours) with its own palette, type and icons, so it reads as related in feel but is its own product.

## First run: seed the profile

The first screen after install asks for a resume and a LinkedIn profile, and builds the profile from both before the person sees an empty form. The model download runs at the same time, so the person is filling in their details instead of watching a progress bar.

1. **Resume.** Drop a PDF, Word or Markdown file. CeeVee extracts the text and the local model maps it to profile sections (experience, education, skills, projects, certifications), keeping the original wording of each bullet.
2. **LinkedIn.** Three ways in, offered in this order:
   - **Profile PDF:** on their LinkedIn profile the person picks More, then Save to PDF, and drops the file in. Fastest, and works for everyone.
   - **LinkedIn data export:** the full archive from LinkedIn's Settings, Data privacy, Get a copy of your data. Its CSV files (positions, education, skills, certifications, projects, languages) give exact dates and titles with no AI guessing. LinkedIn can take up to a day to email it, so CeeVee lets the person carry on and import it later.
   - **Profile URL:** the person pastes their profile link and CeeVee opens it in the automation browser, where they sign in to LinkedIn themselves, then reads the page once. Offered as a convenience only, because LinkedIn's terms forbid automated scraping; CeeVee reads only the person's own profile and never other people's.
3. **Merge and review.** Entries from both sources are matched by company, title and dates. Duplicates merge into one entry, and conflicts (different dates, a title in one source only) are shown side by side for the person to pick. Nothing is saved to the profile until they confirm.
4. **Fill the gaps.** A short follow-up asks for what neither source holds but auto-fill needs: work authorization, notice period, salary range, willingness to relocate, and links. These go into the answers store.

Every step can be skipped, and both imports stay available later from the Profile screen to refresh the profile.

## Resumes and cover letters

Every person has one base resume, and every application gets its own copy of the resume plus its own cover letter, made from the base and tailored to that posting.

| Document | Made from | Used for | Edits affect |
| --- | --- | --- | --- |
| Base resume | The profile, in the person's chosen theme, section order and wording | General use, quick applications, and the starting point for every tailored copy | The base only; existing applications keep their own copies |
| Application resume | A snapshot of the base when the application is created, then tailored to the posting: summary rewritten, bullets reordered and reworded toward the requirements, irrelevant entries hidden | The file auto-fill uploads and the email attaches for that application | That application only |
| Application cover letter | The profile and the posting, in the tone and language the person picks | Uploaded or pasted by auto-fill, attached to the email | That application only |

Rules that hold for all three:

- **Profile facts only.** Tailoring may reorder, shorten and reword, but never adds an employer, title, date, skill or number that is not in the profile. Requirements the profile does not meet are listed as gaps.
- **Frozen once sent.** When an application is marked Sent, its resume and cover letter are locked as the exact versions submitted, so the person can always see what the employer received. Further edits make a new version.
- **Version history and undo** on every document, with what caused each version (AI draft, chat edit, manual edit).
- **Pull from base.** If the person improves the base resume or profile later, an open application can pull those changes into its copy with one click, shown as a diff first.
- **Export** each document as PDF or DOCX, named by company and role, and download all of an application's files at once.

**The profile learns from the conversation.** The chat is where people remember things their resume left out ("I also led the migration to AWS", "I'm fluent in Spanish", "I can't relocate"). The profile is the single source of truth, so anything new said in any chat (refine, onboarding, drafting an email reply) flows back into it:

1. After each chat turn, the model checks the message for new or changed facts: a skill, an achievement with a number, a role detail, a date correction, a preference such as salary or location.
2. Each one appears as a small card in the chat, for example "Add to Acme Corp: Led AWS migration of 40 services", showing where it would go and the exact wording.
3. The person accepts, edits or dismisses it. Accepted facts are saved to the profile at once, with a note of which conversation they came from, and can be used in this application's documents straight away.
4. A setting can switch low-risk changes (new skills, preferences) to save automatically, with an undo toast. Changes to employers, titles and dates always ask.

This keeps the "profile facts only" rule honest: the AI can use what the person told it, but only after it has become a profile fact the person approved. Changes to the profile then show up in the base resume and as a "Pull from base" offer on open applications.

## Job categories

A category is a named target the person is aiming for, such as "Senior product manager, fintech, remote US". Every posting and application belongs to one, and each category steers search, scoring, documents and reporting. Most people have 1 to 4.

| Field | Examples | Matching |
| --- | --- | --- |
| Name | "Fintech PM", "Data engineering, healthcare" | n/a |
| Industry | Fintech, healthcare, SaaS, public sector | Any of a list |
| Function | Product, engineering, design, sales, operations | Any of a list |
| Level | Entry, mid, senior, staff or principal, manager, director, VP | A range |
| Job titles | "Product Manager", "Product Owner" | Keywords, with synonyms the model suggests |
| Location and work mode | Seattle area, remote US, hybrid up to 2 days | Places and radius; remote, hybrid, on-site |
| Pay | At least $150,000 base | Minimum, in the posting's currency |
| Employment type | Full-time, contract, part-time | Any of a list |
| Company size and stage | Startup, 50 to 500 people, public | Any of a list |
| Must-have and deal-breaker keywords | Must: "payments"; never: "crypto", "on-call" | Required or excluded words |
| Priority | 1 to 3 | Orders the dashboard and agent |

Every field is optional except the name. Each field can be marked required or preferred: required fields filter postings out, preferred ones only raise the score.

What a category drives:

- **Creating one.** From scratch, from a posting the person liked ("make a category like this"), or suggested by the model from the profile during onboarding.
- **Finding jobs.** Each category becomes a saved search for job feeds and the daily job agent.
- **Scoring.** Postings are scored against the profile and against the category's required and preferred fields, and the reasons show both.
- **Base resume per category.** A category can have its own variant of the base resume (for example a product-focused and a data-focused version), which its applications are tailored from. Default tone and cover letter style can also be set per category.
- **Tracking.** The board, table and dashboard filter by category, and the funnel compares them, so the person can see which direction gets responses.

**Categories drive every search.** Searches are not set up separately: each category is the search. Job feeds and the daily agent run one query per active category, built from its titles, location, level and keywords, and every result is filed under the category that found it.

**Companies are first-class.** Every posting and application links to a company record, so the person can browse jobs by company as well as by category: all open roles at a company, every application ever sent there, the emails from its recruiters, and notes on it. A company can be put on a watchlist ("target companies"), which adds its careers page to the searches and flags its new postings on the dashboard. A company can also be blocked, which hides its postings everywhere.

In the database this adds `job_category`, `company` and a `posting_category` link table; see Data model.

## Job sources

Each category searches three kinds of source: company career sites, job boards, and job posting services. Results from all three are merged, deduplicated and scored in one list. Only the search terms leave the computer, never the profile.

| Kind | Sources | How CeeVee reads them | Cost and limits |
| --- | --- | --- | --- |
| Company career sites | Greenhouse, Lever, Ashby, SmartRecruiters, Workable and Recruitee job boards; Workday career sites; any careers page that publishes schema.org JobPosting data | The public job-board feeds these systems publish for each employer, polled for every watchlisted company and every company seen in past results. A plain careers page is read for its JobPosting data, following robots.txt | Free. Best source: postings are fresh and apply links go straight to the employer |
| Job boards | Indeed, LinkedIn, Glassdoor, ZipRecruiter, Dice, Wellfound; free ones such as USAJOBS, Remotive, We Work Remotely, Hacker News "Who is hiring" | Free boards through their official API or RSS. The big boards have no open API and forbid scraping, so CeeVee reads their **job alert emails** through the connected inbox: the person sets alerts on those sites (CeeVee suggests the search terms for each category) and every alert becomes postings in CeeVee | Free; the alert route is allowed by the boards' terms |
| Job posting services | JSearch (Google for Jobs, which covers LinkedIn, Indeed and others), Adzuna, TheirStack | Their search APIs, one query per active category per day | Paid per search; see below |

How results are handled:

- **Deduplicated** by company, title and location, and by the link without tracking parameters. When the same job appears on a board and on the company's own site, CeeVee keeps the company link for applying.
- **Linked to a company.** New companies are created as they appear, and their careers system is detected so later searches can poll it directly.
- **Filed and scored per category.** A posting matching two categories appears in both, with a score for each.
- **Scheduled** once a day per category while the app or its tray icon is running, plus a Refresh button. Each source has a polite rate limit and back-off.
- **Expired postings** are marked closed when they drop out of the source.

**Posting service costs.** Posting services charge per search. At $19.99 one-time, Turtlebox keeps about $18.50 per sale after payment fees, while a relay holding shared API keys would cost about $10 per user per year at a few cents a day. **Recommendation:** v1 relies on the free sources (company career sites, free boards and alert emails), which cover most postings, and paid posting services are optional with the person's own API key. A relay can come later as a paid add-on.

### Which sites and systems come first

Chosen by scale and coverage of US postings. Applicant tracking systems are ranked by how many employers use them, because each one CeeVee supports opens every employer on it for both search and auto-fill. Job boards are ranked by US audience.

| Priority | Applicant tracking system | Coverage | Search | Auto-fill |
| --- | --- | --- | --- | --- |
| 1 | Workday | Over 40% of the Fortune 500 (up from 39.2% in 2025); 17.5% of a 13,130-company sample | Career-site job feed per employer | v1. Hardest: multi-page, each employer needs its own account, which the person creates |
| 2 | Greenhouse | 19.0% of the 13,130-company sample; the leader among tech and mid-size employers | Public job-board API | v1. Single page, easiest |
| 3 | Lever | 15.6% of the sample | Public job-board API | v1 |
| 4 | iCIMS | 14.4% of the sample; large employers, retail and healthcare | JobPosting data on career pages | v1 |
| 5 | Ashby | Fastest-growing; largest share among recruiting teams in one 2026 integration index | Public job-board API | v1. Simple, so cheap to add |
| 6 | SAP SuccessFactors | 12% of the Fortune 500 | JobPosting data on career pages | v1.x |
| 7 | Oracle (Taleo and Oracle Recruiting) | Large enterprises and government contractors; declining | JobPosting data on career pages | v1.x |
| 8 | SmartRecruiters, Workable, Jobvite | Mid-size employers | Public APIs for the first two | v1.x |
| 9 | Small-business systems: BambooHR, Paylocity, ADP, UKG, Paycom | Many small US employers, few postings each | JobPosting data on career pages | Generic form path in v1; adapters later |

The v1 five (Workday, Greenhouse, Lever, iCIMS, Ashby) together cover roughly two thirds of the sample and most large US employers. Every other site still gets the generic auto-fill path.

| Priority | Job board | US scale | How CeeVee reads it | Release |
| --- | --- | --- | --- | --- |
| 1 | Indeed | Over 70 million monthly US web visitors, 19 million app users (early 2025) | Job alert emails | v1 |
| 2 | LinkedIn Jobs | The largest professional network; most white-collar postings appear here | Job alert emails | v1 |
| 3 | ZipRecruiter | Over 11 million monthly US visitors | Job alert emails | v1 |
| 4 | Glassdoor | Over 7.5 million monthly US visitors; shares listings with Indeed | Job alert emails | v1 |
| 5 | USAJOBS | Every US federal job | Official free API | v1 |
| 6 | Adzuna | Large US aggregator | Official API with a free tier | v1 |
| 7 | Dice, Wellfound, remote boards (Remotive, We Work Remotely) | Tech, startup and remote niches | Alert emails, APIs or RSS | v1.x |

Google for Jobs (through JSearch) stays an optional bring-your-own-key source.

Sources: [Jobscan, Fortune 500 ATS report 2026](https://www.jobscan.co/blog/fortune-500-use-applicant-tracking-systems/); [Pin, ATS market share report, June 2026](https://www.pin.com/blog/ats-market-share-report) (cites the Jobscan 13,130-company figures); [Sensor Tower, leading US jobs brands, Q1 2025](https://sensortower.com/blog/2025-q1-us-leading-3-Jobs---Careers-brands).

## Connections at each company

CeeVee shows, on every posting and company, who the person knows there now and who used to work there, and helps them ask for a referral. LinkedIn closed its connections API to outside apps years ago, so CeeVee builds this from data the person owns rather than a live LinkedIn link.

| Source | What it gives | How |
| --- | --- | --- |
| LinkedIn data export (Connections file) | Every first-degree connection: name, profile link, current company and title, date connected, email when the connection shares it | Imported with the onboarding export, and again whenever the person drops in a fresh export (CeeVee reminds them monthly) |
| The person's own work history | Former colleagues: connections now elsewhere who overlapped with the person at a past employer, and connections whose current company is one the person used to work at | Matched against the profile's experience entries |
| Connected inbox | People the person has emailed at a company's domain, including recruiters and past interviewers | Read from mail headers already synced for tracking |
| Profiles the person opens | Full work history of one contact, which is what shows "used to work at" | When the person opens a contact's LinkedIn profile in CeeVee's browser, they can press Save to capture that one page. Never automatic or in bulk |
| Manual | Anyone else: a friend, an alum, a recruiter met at an event | Add a contact by hand, or paste a profile PDF |

What the person sees:

- **On each posting and company:** "You know 3 people at Acme: 2 work there now, 1 used to", with each person's title, how they are connected (LinkedIn, former colleague, email) and when they last spoke.
- **In scoring:** a known contact at the company raises a posting's rank, since referrals get far more responses.
- **Referral requests:** the model drafts a short, personal message to a contact about a specific role, from the profile and posting, for the person to send through LinkedIn or email themselves.
- **Outreach tracking:** each ask is logged on the application (asked, replied, referred), with a follow-up reminder.

Limits to be clear about: second-degree connections (friends of friends) are not available outside LinkedIn, and "used to work at" is only complete for contacts whose profile the person has saved or whose history overlaps their own. CeeVee never scrapes LinkedIn or sends messages on the person's behalf.

In the database this adds `contact` (name, profile link, email, source, notes), `contact_role` (contact, company, title, start and end dates, current or past) and `outreach` (contact, application, message, status, dates).

## Architecture

CeeVee is one process: a Rust core that owns the database, model, browser and mail, and a web UI in the system webview that talks to it through Tauri commands and events. Long tasks (drafting, form filling, mail sync) run as background jobs in the core and stream progress to the UI, the way Dossier's job queue does.

| Part | Choice | Why |
| --- | --- | --- |
| App shell | Tauri 2 (decided) | Native installers for Windows (per-user NSIS) and macOS (DMG, arm64), around 10 MB before models, built-in updater and tray |
| UI | TypeScript with SvelteKit (static build) and Tailwind (decided) | Fastest way to reach Dossier-level polish |
| Core | Rust, Tokio, crates split by area: `db`, `ai`, `docs`, `apply`, `mail`, `license` | Each area testable on its own |
| Database | SQLite through `sqlx` with versioned migrations, WAL mode, FTS5 for search | One file in the app data folder, easy to back up |
| Secrets | OS keychain through the `keyring` crate (Credential Manager, macOS Keychain) | OAuth tokens and passwords never sit in the database |
| Local model | llama.cpp through the `llama-cpp-2` crate; Metal on Mac, Vulkan on Windows, CPU fallback | Works with no extra install; GGUF models |
| Default models | A 4B instruct model at Q4 (about 2.5 GB) for 8 to 16 GB machines; an 8B model offered at 16 GB and up; a small embedding model for matching | Chosen by a first-run hardware check; downloaded with progress and checksum |
| Optional AI | Ollama, LM Studio, or an OpenAI-compatible or Anthropic key | For people who want a larger model |
| Structured output | GBNF grammars from JSON schemas | Small local models return valid JSON every time |
| Documents | Typst templates rendered to PDF and SVG preview; DOCX through `docx-rs` | Vector PDFs that ATS parsers read reliably, no Chromium needed |
| Resume import | `pdf-extract` / `lopdf` for PDF, `docx-rs` for Word, then the model maps text to the profile | Same flow as Dossier's import |
| Browser automation | `chromiumoxide` over the Chrome DevTools Protocol, driving Edge (on every Windows 11 PC) or Chrome; Chrome for Testing downloaded if neither exists | Visible window with its own profile, so logins to job sites persist |
| Email | `async-imap` for reading, `lettre` for sending; OAuth 2 with PKCE and a loopback redirect for Gmail and Microsoft | Works with any provider, not only Gmail |
| Licensing | Ed25519-signed license files checked offline, activation through turtlebox.studio | See the licensing section |

**How auto-fill works.** The person clicks Apply on an application. CeeVee opens the posting in the automation window and identifies the applicant tracking system from the page. Known systems (Workday, Greenhouse, Lever, iCIMS, Ashby in v1) get a hand-written adapter that knows their fields and steps. Any other form goes through a generic path: CeeVee reads the form's labels and accessibility tree, the model maps each field to a profile fact, and anything it cannot answer with confidence is left empty and highlighted. Free-text questions are drafted from the profile. Files are uploaded from the tailored PDFs. CeeVee then stops, shows a checklist of what it filled and what needs a person, and waits. It never solves CAPTCHAs, never creates accounts on its own and never presses Submit; when the person submits, CeeVee notices the confirmation page and marks the application Sent.

**How email tracking works.** After the person connects a mailbox, CeeVee syncs the last 90 days of mail headers, then checks every 10 minutes while open. A cheap filter (sender domain matches an application's company, known ATS senders, thread replies to mail CeeVee sent) picks candidate messages; only those reach the model, which labels them and pulls out interview times and questions. Each match is linked to an application and suggests a status change the person confirms with one click. Unmatched job mail lands in a "Needs a look" list, as in Dossier.

## Data model

The SQLite file has 20 core tables in six groups. Jobs can be queried by category and by company, both indexed. Structured AI output (extracted requirements, match results, email labels) is stored as JSON columns next to the prompt version that produced it, so results can be re-run when prompts improve.

| Group | Table | Holds |
| --- | --- | --- |
| Profile | `profile` | Name, contact, summary, location, links, photo |
| Profile | `profile_entry` | Experience, education, projects, certifications, publications, references; one row each with a `kind`, dates, bullets and sort order |
| Profile | `skill`, `language` | Skills with groups; languages with levels |
| Profile | `fact` | Answers for forms: question key, answer, sensitivity flag (EEO and salary answers are never filled without asking) |
| Jobs | `job_category` | A target the person defines: name, industries, functions, level range, titles, locations and work mode, minimum pay, employment types, company size, must-have and excluded keywords, required or preferred per field, priority, active flag, linked resume variant |
| Jobs | `company` | One employer: name, aliases, website and email domains (for matching mail), careers page and ATS, industry, size, location, watchlist or blocked, notes |
| Jobs | `posting_category` | Links a posting to every category it matches, with that category's score and reasons; indexed both ways |
| Jobs | `posting` | A job seen or saved: source, URL, `company_id`, title, location, level, pay, text, score, saved or dismissed, first and last seen; indexed on company |
| Jobs | `application` | One application: posting link, `company_id`, `category_id`, status (Draft, Sent, Screening, Interview, Offer, Accepted, Rejected, Declined, Withdrawn), extracted and match JSON, tone, language, dates, next interview, board order |
| Jobs | `status_event` | Every status change with time and cause (manual, email, auto-fill) for the funnel and timeline |
| People | `contact`, `contact_role` | People the person knows (from the LinkedIn export, email, saved profiles or by hand), and each one's current and past roles linked to companies, so a company shows who works there now and who used to |
| People | `outreach` | Referral asks and networking messages: contact, application, message, status (asked, replied, referred), dates |
| Documents | `document`, `document_version` | Base resume (and per-category variants), tailored resume, cover letter and email per application, with full version history and what caused each version |
| Documents | `chat_message`, `note` | Refine chat and private notes per application |
| Documents | `theme` | Built-in and custom resume themes |
| Automation | `apply_run`, `apply_field` | Each auto-fill attempt: ATS detected, outcome, and every field with its value, source fact and confidence, for review and for improving adapters |
| Mail | `mail_account`, `email_message` | Connected accounts (tokens live in the keychain); messages with direction, thread, linked application, label, summary, suggested status, interview time |
| System | `job`, `setting`, `license_state` | Background tasks with progress; settings; trial start, license and last check |

Migrations ship inside the app and run at start-up, after an automatic backup of the file.

## Licensing, trial and distribution

People download CeeVee free from turtlebox.studio, use every feature for 14 days, then make a one-time purchase of $19.99 there to keep going. There is no subscription, and each license covers 2 machines. After the trial the app stays read-only: the person can still view and export their data, so nobody loses their job search.

1. **Download.** turtlebox.studio offers a signed Windows installer and a notarized macOS DMG for Apple silicon. No account needed. The same public download serves trial users and buyers, because the license, not the download, is what unlocks the app.
2. **Trial.** On first run the app records the start time in the database and the OS keychain and asks turtlebox.studio once for a signed trial token tied to a hashed machine ID. Offline starts still work; the token is fetched on the next connection. Clock rollback is detected by comparing against the latest timestamp seen.
3. **Purchase.** Checkout on turtlebox.studio through FastSpring, which is the merchant of record and so collects and remits US sales tax. FastSpring's license fulfillment calls a license endpoint on the turtlebox.studio AWS backend (the highturtle repo), which creates the key and stores the order; FastSpring emails the key to the buyer, with an "Activate in CeeVee" button (a `ceevee://activate` link that opens the app with the key filled in), the key in plain text, and download links for both platforms.
4. **Activation.** The app sends the key and machine ID once and gets back a license file signed with Ed25519. The public key is built into the app, so every later check is offline. Each license covers 2 machines, and people can free a seat from the website.
5. **Updates.** Covered in the next section. Updates are free for the purchased major version.

On AWS this needs only a small API (license fulfillment webhook, trial tokens, activation and seat release, for example API Gateway with Lambda and DynamoDB) and S3 with CloudFront for installers, update manifests, model files and adapter updates, all added to the highturtle repo.

Code signing is required on both platforms or the installers will be blocked: an Apple Developer ID with notarization ($99 a year) and a Windows certificate (Azure Trusted Signing, about $10 a month, is the cheapest route). Both need identity checks that can take days to weeks, so they should be started before the first build. Model files are downloaded from a CDN controlled by Turtlebox, not from third-party hubs at run time, so the app works the same for every buyer.

## Delivery and updates

How CeeVee gets onto a computer and stays current. Engineering detail is in TECHNICAL_PLAN.md, section 16.

**Getting the app.**

- turtlebox.studio/ceevee has one Download button that picks Windows or Mac from the visitor's browser, with the other platform linked below it. Links point at `dl.turtlebox.studio`, a CloudFront address in front of a private S3 bucket.
- Windows gets a signed per-user installer (`CeeVee_x.y.z_x64-setup.exe`, about 10 MB). It installs without an administrator prompt, which also lets updates install without one. Windows 11 already has the WebView2 runtime the app needs.
- Mac gets a notarized DMG for Apple silicon. If the app is started from the DMG or the Downloads folder, it offers to move itself to Applications, because macOS cannot update an app that runs from there.
- The local model is not in the installer. It downloads on first run while the person sets up their profile (see First run).
- After purchase, the FastSpring email carries the key, a one-click activation link and both download links. A small "My license" page on turtlebox.studio (sign in with an emailed link, no password) shows the key, both seats with a "Free this seat" button, and the downloads again, so a lost email is never a support ticket.

**Staying current.**

- The app checks for an update at launch and every 6 hours while it runs. Updates download in the background and are verified against a signature before anything is installed.
- The person sees "Update ready: restart to finish", with the release notes. CeeVee never restarts on its own, and holds the install while an auto-fill, draft or mail sync is running. Postponed updates install at the next quit.
- Every update is free within the purchased major version, and updates are not tied to the license: trial and expired-trial installs get fixes too, so read-only data stays readable. When a new major version ships, older versions see an "upgrade available" note instead of an automatic update.
- Before any update that changes the database, the app makes a backup, so the person can go back to the earlier version if needed.

**Release channels.**

| Channel | Who | What they get |
| --- | --- | --- |
| Stable | Everyone (default) | Releases that have run on Beta first, rolled out in steps |
| Beta | Opt-in in Settings, and the 20 to 50 launch testers | Release candidates a week or so ahead of Stable; also receives each Stable release |
| Internal | Turtlebox only | Every build from main, for testing on real machines |

**Staged rollout.** A Stable release first reaches 10 percent of installs, then 50, then 100 over about a week, with the step chosen by a random ID that each install keeps on the machine (nothing identifying is sent). If crash reports or support mail show a problem, the rollout is paused in one change to the manifest, and the fix ships as the next version. Updates only move forward; nobody is automatically downgraded.

**Content that updates without a release.** Three kinds of files update on their own schedule, each listed in a manifest signed by Turtlebox and checked by the app before use:

| Content | How often it changes | How it updates |
| --- | --- | --- |
| Auto-fill adapters and job-alert email parsers | Whenever a job site changes its form | Checked daily, applied immediately, falls back to the version built into the app if a file fails its check |
| Model files | A few times a year, when a better small model is worth it | Offered, not forced (it is a multi-gigabyte download); downloads in the background and replaces the old model only after it is verified; the old file is then deleted |
| Model settings (which model each hardware tier gets) | With model changes | Part of the model manifest |

Prompts change with the code that reads their output, so they ship only in app releases.

**What it costs to run.** The fixed costs are Apple's $99 a year and about $10 a month for Windows signing. Bandwidth is the variable cost, and the model download dominates it: about 2.5 GB per new install, which is roughly $0.20 per install at CloudFront's standard US rate, against about 10 MB for the app and a few MB per update. Because trial users download the model too, every trial costs about that much whether or not it converts. CloudFront's flat-rate plans or a zero-egress store such as Cloudflare R2 for model files alone would cut this; this is listed under Open below.

## Roadmap

Version 1 takes about 27 weeks for one or two developers, in milestones that each end with something runnable on both platforms. Durations are estimates and assume work starts in late November 2026.

1. **Foundation (weeks 1 to 3).** Tauri app with sidebar and light and dark themes, SQLite with migrations, CI building signed and notarized Windows and macOS installers and publishing them to the Internal channel, background job runner.
2. **Local AI (weeks 4 to 6).** llama.cpp linked in, hardware check, model download with progress, grammar-constrained JSON, a benchmark on an 8 GB Windows laptop and an M1 MacBook Air.
3. **Profile and documents (weeks 7 to 11).** First-run onboarding with resume and LinkedIn import and merge review, profile editor, 4 themes, PDF and DOCX export, new application from a posting with match and gaps, tailored resume and cover letter, refine chat and versions.
4. **Tracking and search (weeks 12 to 17).** Board, table, dashboard funnel and weekly chart, follow-up reminders, export, import and backups. Job categories, companies and job search across career sites and free boards, plus LinkedIn connections at each company.
5. **Auto-fill (weeks 18 to 23).** Browser control, adapters for Greenhouse, Lever and Ashby (simplest) then Workday and iCIMS (largest coverage), then the generic form path; review checklist and submit detection.
6. **Email and launch (weeks 24 to 27).** IMAP with Gmail and Outlook, reply labelling and linking, job alert email parsing, trial and licensing, updater with Beta and Stable channels and staged rollout, signed adapter and model manifests, turtlebox.studio download, checkout and "My license" pages, beta with 20 to 50 testers.

After v1: the background job agent that drafts applications for review, calendar sync and more ATS adapters (v2).

## Risks and open questions

| Risk | Effect | Plan |
| --- | --- | --- |
| Dossier is AGPL-3.0 | Copying any of its code, prompts, CSS or images would force CeeVee to be open source | Clean-room build: use it only as a product reference; record this rule in the repo |
| Gmail's full-mail scope is "restricted" | A paid app that reads Gmail needs Google verification and a yearly third-party security assessment (often several thousand dollars) | Launch with IMAP app passwords and Microsoft OAuth; budget the Google assessment or use the narrower metadata plus labels approach for v1 |
| Job sites change forms and add bot checks | Adapters break; some sites forbid automation | Adapters versioned and updatable without a full release; human always submits; fall back to copy-paste helpers |
| Small local models write weaker drafts | Lower quality than hosted models | Grammars for structure, short focused prompts per task, optional larger or hosted model |
| 8 GB Windows laptops without a usable GPU | Slow drafts (a minute or more) | Show progress, run in background, recommend the smallest model |
| Signing accounts take weeks to approve | No installable builds for testers | Start Apple Developer and Azure Trusted Signing enrollment before week 1 |
| Losing the updater signing key | Installed copies can never be updated again | Key kept in two offline backups as well as in CI; rotation path documented in the technical plan |
| A bad update reaches everyone | Broken app for paying users | Beta first, then 10/50/100 percent rollout that can be paused; database backup before migrations |
| Model download bandwidth on trial installs | Hosting cost grows with trials, not sales | Watch cost per install; move model files to a flat-rate or zero-egress host if needed |
| Trial and license bypass | Lost sales | Signed licenses and trial tokens are enough deterrent; avoid heavy DRM that hurts paying users |

### Decisions log

| Date | Decision |
| --- | --- |
| 2026-11-10 | Tauri 2 with a TypeScript (SvelteKit) UI and a Rust core |
| 2026-11-10 | turtlebox.studio runs on AWS (highturtle repo); FastSpring for payments |
| 2026-11-10 | $19.99 one-time purchase after the 14-day trial, no subscription, 2 machines per license |
| 2026-11-10 | Job sites and applicant tracking systems chosen by scale and coverage (see Job sources) |
| 2026-11-10 | v1 includes category-driven job search across all three source kinds |
| 2026-11-10 | First audience: English-speaking job seekers in the United States (US English UI, US Letter, MM/DD/YYYY, USD, US form answers such as work authorization and voluntary EEO, veteran and disability questions, always asked and never guessed) |

### Open

- Relay for paid posting services, or bring-your-own key (recommended: bring-your-own key in v1).
- Where model files are served from: CloudFront like everything else (simplest), CloudFront on a flat-rate plan, or Cloudflare R2 for model files only (recommended to price once trial volume is known; start on CloudFront).
- Whether to also offer an MSI installer for IT-managed Windows PCs (recommended: not in v1; the per-user installer updates without an administrator).
- Delivery and update design (section above) proposed 2026-11-11, awaiting Howard's review.
