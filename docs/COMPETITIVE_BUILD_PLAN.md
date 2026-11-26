# Competitive build list: beating Simplify

Last updated 2026-11-26. Source: the [Simplify competitive review](https://claude.ai/artifact/35wvxXYEgAuJTfzJHhmHfh). This is a plan only. No feature in it is started.

The rule for every item: CeeVee does it **better** (more control, more private, more accurate) and **simpler** (fewer steps, fewer settings, fewer screens). Each item names what Simplify does, what CeeVee does instead, the size, and where it sits in the build order.

Sizes: **S** is under a week, **M** is 1 to 3 weeks, **L** is over 3 weeks. Sizes are rough and assume the core app in docs/PLAN.md exists.

## Principles that apply to every item

1. **One place for each thing.** Every feature lives in the chat-anchored home, the job-type pages or the account settings modal from the Turtlebox UX framework. No new top-level screens.
2. **Default to the safe, quiet option.** The app does the work and shows a short result. Settings exist but are hidden until asked for.
3. **Everything is local first.** A feature that needs a server or a hosted model is optional and says exactly what leaves the machine.
4. **Show the proof.** Every automated action leaves a record the user can open: what was filled, sent, or drafted.
5. **Light and dark themes, dense lists, keyboard friendly.** Follow turtlebox-ui components. Do not build one-off widgets.

## Build order at a glance

| Phase | Theme | Items |
|---|---|---|
| 1 | Make the first ten minutes work | 1 Free autofill tier, 2 Starter job feed, 3 Blank-field check, 4 Application record |
| 2 | Match Simplify where it is strongest | 5 Companion browser extension, 6 Explained matches, 7 Resume score and tailoring, 8 Import from Simplify |
| 3 | Go past what Simplify can do | 9 Reply-aware nudges, 10 Controlled automation, 11 Interview practice, 12 Network and referrals |
| 4 | Reach and polish | 13 Better writing option, 14 Public job lists, 15 Mobile companion |

Phase 1 is the launch blocker. Phase 2 closes the gaps that decide first impressions. Phase 3 is where CeeVee pulls ahead. Phase 4 is growth.

## Phase 1: make the first ten minutes work

### 1. Free autofill tier (Decision, then S)

- **Simplify:** Copilot autofill is free forever, with no application cap. Payment is pitched later.
- **CeeVee, better and simpler:** keep the 14-day full trial, and after it ends keep autofill, the tracker and the document editor free. A license unlocks search, email tracking, drafting and the higher automation levels. One rule, easy to explain: "Fill forms free. Pay once for the rest."
- **Needs a decision from Howard:** free autofill after the trial, a 30-day trial, or neither. See docs/QUESTIONS.md.
- **Fits the UX:** the trial banner and license state live in the account menu and the Billing tab of the settings modal. No locked screens, only a quiet "Upgrade" row beside the features that need it.
- **Depends on:** license store backend.

### 2. Starter job feed (M)

- **Simplify:** a large curated board refreshed hourly, plus public GitHub lists. Some users run out of jobs.
- **CeeVee, better and simpler:** during onboarding the user picks a job type (for example "Product manager, senior, remote") and the app shows real openings in under a minute, drawn from ATS feeds (Greenhouse, Lever, Ashby, Workday where available), board alert emails and the optional API sources. Results are grouped by job type and company, not one endless board. No empty first search.
- **Better:** every job shows why it was shown (matched title, level, location, salary) and can be dismissed with one key so the feed learns.
- **Fits the UX:** job types are in the left nav. The feed is the middle pane of the three-pane layout, with the chat on one side and the job detail on the other.

### 3. Blank-field check before Submit (S)

- **Simplify:** autofill leaves blanks without warning, which is a top complaint.
- **CeeVee, better and simpler:** before any submit, a single checklist shows every field as Filled, Needs you, or Low confidence, with the answer source for each. "Fix all" is one click and the app asks only for what is missing. Nothing is submitted while a required field is empty.
- **Fits the UX:** appears as a compact card in the chat and the job detail pane.

### 4. Application record (S)

- **Simplify:** Application History keeps the exact resume, letter and answers sent.
- **CeeVee, better and simpler:** the same record, kept on the user's machine, written automatically when a form is filled and again at submit. It also stores the job post text as it was on the day, since postings disappear. One "Open record" button per application. Already near-free because everything is in SQLite.
- **Fits the UX:** a tab in the job detail pane.

## Phase 2: match Simplify where it is strongest

### 5. Companion browser extension (L)

- **Simplify:** the Copilot extension fills forms inside the user's own, already signed-in browser. This is its biggest ease-of-use edge.
- **CeeVee, better and simpler:** a thin extension (Chrome, Edge, Brave, Firefox) that fills in the user's own browser but takes everything from the desktop app over a local connection. The profile, answers, policy and records stay on the machine. The extension stores no personal data and makes no network calls to us. The separate automated browser window stays for scheduled and hands-off runs.
- **Better:** one automation policy applies in both places, and the extension shows the same blank-field check as item 3.
- **Simpler:** pairing is one click ("Connect to CeeVee") with no account or code to type.
- **Risks:** store review for broad page permissions. Ask only for the ATS domains and listed job sites.
- **Fits the UX:** pairing and status live in the Browser tab of the settings modal.

### 6. Explained matches (M)

- **Simplify:** match scores and dealbreaker filters, with no reason given.
- **CeeVee, better and simpler:** each job gets a plain sentence ("Senior title, remote, pays above your floor; missing: SQL") and dealbreakers are set once per job type in plain words. The local model writes the reason, and the rules (remote, salary, visa) are applied before the model so they are never wrong.
- **Simpler:** no numeric score unless the user turns it on. Three states: Strong, Possible, Skip.

### 7. Resume score and tailoring (M)

- **Simplify:** free builder with an ATS-style score; line-by-line tailoring is paid.
- **CeeVee, better and simpler:** a base resume per job type and an automatic tailored copy per application, as already planned. The review is one screen with changes highlighted and an Accept all button. The keyword gap list says which skills the posting wants and whether the resume already shows them. Never invents experience: every change must cite a line from the user's own profile, and anything unsupported is shown as a question.
- **Better:** works offline with the local model, and can use a hosted model if the user opts in (item 13).
- **Fits the UX:** edited through chat, with a side-by-side preview in the right pane.

### 8. Import from Simplify (S)

- **Simplify:** none. Lock-in is the default.
- **CeeVee, better and simpler:** accept the Simplify data export, resume files and a tracker CSV, and fill the profile, saved answers and application list in one step. Same importer handles LinkedIn and CSVs. Lowers the cost of switching to zero.
- **Fits the UX:** part of onboarding and a button in the Data tab of the settings modal.

## Phase 3: go past what Simplify can do

### 9. Reply-aware nudges (M)

- **Simplify:** the Gmail link stores only message metadata and short previews.
- **CeeVee, better and simpler:** reads recruiter replies (Gmail, Outlook or IMAP), links each to its application, moves the status and then acts: interview invite becomes a calendar hold and prep notes, rejection becomes a closed card, a recruiter question becomes a drafted reply. All of it is processed on the machine and the user approves sends according to the automation level.
- **Simpler:** the user never files an email by hand.
- **Depends on:** Google verification (pending) or the IMAP fallback.

### 10. Controlled automation (M)

- **Simplify:** two modes. Copilot never submits, and Autopilot submits up to 20 applications a week with no review step.
- **CeeVee, better and simpler:** the planned five levels, but presented as one slider per job type with plain names (Manual, Assisted, Review before submit, Auto with limits, Full auto) and a daily cap. Every auto-submit is logged to the application record and can be paused with one button from the tray.
- **Better:** guard checks stop a submit when a field is blank, a salary or location question has no saved answer, or the job no longer matches. Hard denials stay (LinkedIn messaging, Easy Apply, CAPTCHAs, account creation).
- **Simpler:** the default is Assisted. Nothing is auto-submitted until the user raises the level.

### 11. Interview practice (M)

- **Simplify:** free AI mock interviews.
- **CeeVee, better and simpler:** practice from the real job post and the user's own resume, using the local model, so there is no cost and nothing leaves the machine. Triggered from an interview invite (item 9) with one button on the application card. Notes from each session stay with the application.

### 12. Network and referrals (M)

- **Simplify:** finds referral contacts and drafts outreach.
- **CeeVee, better and simpler:** uses the user's own LinkedIn connections export and email contacts to show "people you know at this company" on each job. Drafts a short ask and leaves sending to the user. No scraping and no automated messaging, which keeps accounts safe.

## Phase 4: reach and polish

### 13. Better-writing option (S)

- **Simplify:** hosted AI for all paid writing.
- **CeeVee, better and simpler:** a single switch in the AI tab, "Use a stronger online model for writing", with a plain list of what is sent (job text and the section being edited, never the full profile or email). Off by default. Answers the local model's quality gap without giving up the privacy story.

### 14. Public job lists (M)

- **Simplify:** GitHub lists such as Summer 2027 internships, about 48K stars, feed its brand.
- **CeeVee, better and simpler:** publish a small number of high-quality lists in areas we serve, generated from the same feed as item 2, each ending with a link to CeeVee. Marketing task more than a product feature; schedule after the feed is reliable.

### 15. Mobile companion (L, optional)

- **Simplify:** iOS app for jobs, Autopilot and Application History.
- **CeeVee, better and simpler:** a read-only view of jobs and application status on the phone, synced through a local-network pair or the user's own cloud drive, with no CeeVee server. Build only if user demand shows up after launch.

## Not building

- **Cloud-hosted profiles and employer matching.** This is the thing users dislike about Simplify.
- **Weekly or monthly subscriptions.** Pricing stays one-time.
- **Unlimited unattended auto-apply.** It floods employers and hurts users. Caps and guard checks stay.
- **Public support forum with user messages.** Support stays private.

## Fit with the Turtlebox UX redesign

| Turtlebox element | Items that use it |
|---|---|
| Chat-anchored home | 3, 6, 7, 9, 11, 13 (cards and drafts appear in chat) |
| Job types in left nav | 2, 6, 10 (feed, dealbreakers and automation level per type) |
| Three-pane chat, list, detail | 2, 3, 4, 7 |
| Account menu and tabbed settings modal | 1 (Billing), 5 (Browser), 8 (Data), 13 (AI) |
| Dense lists and toggles | 2, 10 |
| Light and dark themes | all |

Build the new components in turtlebox-ui once and reuse them in CeeVee and Golden: the checklist card (item 3), the record viewer (item 4) and the level slider (item 10) are likely candidates.

## Measures of success

- First search shows at least 20 relevant jobs within 60 seconds for the top 10 job types.
- Blank required fields at submit: zero.
- Autofill success on Greenhouse, Lever, Ashby, Workday: measured in CI against saved forms, with a published floor in evals/thresholds.json.
- Time from install to first filled application: under 10 minutes.
- Trial to paid conversion and refund requests: refund requests should stay below 2%.

## Open questions for Howard

1. After the 14-day trial, should autofill stay free (item 1), or should the trial be longer?
2. Is a companion browser extension (item 5) in scope for v1, or v1.1? It is the largest item and the biggest ease-of-use gap.
3. Should the optional stronger online model (item 13) go through Anthropic's API, a key the user provides, or both?
4. Is a read-only mobile companion (item 15) wanted at all?
