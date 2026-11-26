# Competitive build list: beating Simplify

Last updated 2026-11-26 (items 1, 5, 13 and 15 decided). Source: the [Simplify competitive review](https://claude.ai/artifact/35wvxXYEgAuJTfzJHhmHfh). This is a plan only. No feature in it is started.

The rule for every item: CeeVee does it **better** (more control, more private, more accurate) and **simpler** (fewer steps, fewer settings, fewer screens). Each item names what Simplify does, what CeeVee does instead, the size, and where it sits in the build order.

Sizes: **S** is under a week, **M** is 1 to 3 weeks, **L** is over 3 weeks. Sizes are rough and assume the core app in docs/PLAN.md exists.

Item 12, the network builder, was added on 2026-11-26 at Howard's request.

## Principles that apply to every item

1. **One place for each thing.** Every feature lives in the chat-anchored home, the job-type pages or the account settings modal from the Turtlebox UX framework. No new top-level screens.
2. **Default to the safe, quiet option.** The app does the work and shows a short result. Settings exist but are hidden until asked for.
3. **Everything is local first.** A feature that needs a server or a hosted model is optional and says exactly what leaves the machine.
4. **Show the proof.** Every automated action leaves a record the user can open: what was filled, sent, or drafted.
5. **Light and dark themes, dense lists, keyboard friendly.** Follow turtlebox-ui components. Do not build one-off widgets.

## Build order at a glance

| Phase | Theme | Items |
|---|---|---|
| 1 | Make the first ten minutes work | 1 Trial and license, 2 Starter job feed, 3 Blank-field check, 4 Application record |
| 2 | Match Simplify where it is strongest | 5 Companion browser extension, 6 Explained matches, 7 Resume score and tailoring, 8 Import from Simplify, 12a Contacts and imports |
| 3 | Go past what Simplify can do | 9 Reply-aware nudges, 10 Controlled automation, 11 Interview practice, 12b to 12e Network builder |
| 4 | Reach and polish | 13 Better writing option (deferred), 14 Public job lists, 15 Mobile companion (deferred) |

Phase 1 is the launch blocker. Phase 2 closes the gaps that decide first impressions. Phase 3 is where CeeVee pulls ahead. Phase 4 is growth.

## Phase 1: make the first ten minutes work

### 1. Trial and license (Decided: 14 days, then fully locked, S)

- **Simplify:** Copilot autofill is free forever, with no application cap. Payment is pitched later.
- **Decision (Howard, 2026-11-26):** keep the 14-day full trial, then lock the app until a license is bought. No permanent free tier. Pricing stays $19.99 one-time.
- **What this means for the plan:** CeeVee cannot win on free entry, so it has to win the first 14 days. Items 2, 3 and 4 (starter feed, blank-field check, application record) become launch blockers, and onboarding should reach a real filled application within the first session.
- **Better and simpler:** the trial banner shows days left and what will lock, in plain words, and the user's data is never held back. The app opens read-only after the trial, so the user can still view and export everything (the licensing work already plans read-only enforcement). The refund rule is shown before purchase, which answers Simplify's refund complaints (open Q22).
- **Fits the UX:** the license state lives in the account menu and the Billing tab of the settings modal. A quiet "Buy" row, no locked-screen walls.
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

### 5. Companion browser extension (Decided: in v1, L)

- **Simplify:** the Copilot extension fills forms inside the user's own, already signed-in browser. This is its biggest ease-of-use edge.
- **CeeVee, better and simpler:** a thin extension (Chrome, Edge, Brave, Firefox) that fills in the user's own browser but takes everything from the desktop app over a local connection. The profile, answers, policy and records stay on the machine. The extension stores no personal data and makes no network calls to us. The separate automated browser window stays for scheduled and hands-off runs.
- **Better:** one automation policy applies in both places, and the extension shows the same blank-field check as item 3.
- **Simpler:** pairing is one click ("Connect to CeeVee") with no account or code to type.
- **Decision (Howard, 2026-11-26):** ships in v1. Start it early because it is the largest item and store review time can delay launch; submit to the stores as soon as a pairing build works.
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

### 12. Network builder (L, built in five steps)

- **Simplify:** a referral finder and outreach drafts for a target company. It does not keep a network. There is no contact list, meeting log, reminder or relationship history.
- **CeeVee, better and simpler:** a personal network manager that lives next to the job search and uses it. It is a people list with the context that makes it useful, kept on the user's machine. No LinkedIn scraping, no automatic connection requests or messages, and no contact data uploaded anywhere.
- **Why it is more than a feature:** most hires still come through people. A job search app that only tracks applications ignores the biggest source of interviews. It also gives CeeVee a reason to be opened every week after the job is found.
- **Where it lives:** a Network entry in the left nav beside Job types, using the same three panes (list, person, chat).

The five steps are built in order. Each is useful alone.

#### 12a. Contacts and imports (M, Phase 2)

- One people list with name, company, title, how you know them, tags, links and the date of last contact.
- **Imports, all local:** the LinkedIn data export (Connections file), Google and Outlook contacts through the optional sign-ins, vCard and CSV, and a "people I have emailed" suggestion list built from the mailbox sync.
- **Simpler:** drop a file or click Connect, and the app dedupes and merges. Re-importing a newer LinkedIn export shows who changed jobs or titles since last time.
- **Better:** every contact links to a company, so each job and company page shows "people you know here".

#### 12b. Relationship notes and follow-up reminders (M, Phase 3)

- A short note per person, plus a timeline of every touch (email, meeting, note). Notes can be dictated into chat ("Met Dana at the PM meetup, she is hiring in Q1").
- Three tiers: Close, Warm, Weak, each with a default follow-up cadence the user can change in one place. A weekly list of the five people most worth contacting, shown in the chat home and the tray. Snooze or done with one key.
- **Better:** reminders say why now ("Dana's company posted a PM role you saved") instead of a bare date.
- **Simpler:** no manual dates. The app proposes the next touch and the user accepts or skips.

#### 12c. Meetings and calendar (M, Phase 3)

- Connect Google or Outlook calendars, or an ICS file. Calendar events that include a known contact are linked automatically.
- **Before:** a one-screen prep brief (last notes, mutual context, open asks, their company's open jobs). **After:** a prompt to log the outcome in one sentence, and a drafted thank-you or follow-up the user can send.
- Schedule a coffee chat from the person page by proposing times from the user's free slots, as an email draft.
- Informational interviews and coffee chats are tracked as a type of meeting, so progress can be seen as a pipeline.

#### 12d. Warm-intro paths and outreach (M, Phase 3)

- For any target company or job, rank the people who can help: direct contacts there, former colleagues from the user's own work history, alumni of the same schools, and contacts whose past employers overlap with that company's team. Uses only the data the user already has.
- Each path ends in a drafted ask: introduction request, informational chat, reconnect note, thank-you, or referral request. The user edits in chat and sends. Email can send under the automation level the user chose. Anything for LinkedIn is copied to the clipboard or opened for the user to send, never automated.
- **Better:** shows the chance of a reply honestly ("Close, last spoke 3 weeks ago" or "Weak, not contacted in 2 years") and suggests who to ask first.
- **Referrals:** a contact can be attached to an application as the referrer, so the outcome of referrals is visible.

#### 12e. Network pipeline, goals and giving back (S to M, Phase 3 to 4)

- **Pipeline:** each outreach moves through Identified, Reached out, Talking, Met, Followed up. Same board style as the application tracker.
- **Weekly goal:** an optional target ("3 conversations a week") with a quiet progress bar. No streak pressure and no guilt copy.
- **Give back:** a list of "people I could help" (intros, feedback, job leads) so the user builds goodwill, not just asks. Nudges appear when a saved job or news item fits a contact.
- **Events:** a simple list of meetups and conferences with who is attending, added by hand or from calendar invites.
- **Health view:** how many active relationships, who has gone cold, and which target companies have no contact yet.

#### What stays out

- No scraping of LinkedIn, no auto connection requests or messages, no browser automation against LinkedIn messaging. This also stays inside the automation hard denials in docs/PLAN.md.
- No cloud contact sync and no shared contact database. Contacts belong to the user.
- No scoring of people's value. Tiers are the user's own labels.

#### Fits the UX

- **Left nav:** Network, with views for People, Companies and Pipeline.
- **Three panes:** list of people, person detail (notes, timeline, linked jobs), and chat with commands like "who do I know at Stripe?" or "remind me to follow up with Dana after her launch".
- **Settings modal:** a Connections tab for LinkedIn export import, Google and Outlook contacts and calendars, and the reminder cadence.
- **Chat home:** the weekly five people to contact and the day's meeting briefs appear as cards.
- **Shared components:** the person row and note timeline are good candidates for turtlebox-ui, since Golden also manages people.

#### Data and size

- Extends the contacts and companies tables in the approved schema. Add interaction, reminder, and meeting tables as needed; confirm against the 37-table design before building.
- Rough total: 12a M, 12b M, 12c M, 12d M, 12e S to M. About 8 to 10 weeks of work in all, and 12a plus the basic company view can ship well before the rest.

## Phase 4: reach and polish

### 13. Better-writing option (Deferred, S)

- **Decision (Howard, 2026-11-26):** not now. Local model only for v1. Revisit when evals show the writing gap hurts users; the design below is the starting point.

- **Simplify:** hosted AI for all paid writing.
- **CeeVee, better and simpler:** a single switch in the AI tab, "Use a stronger online model for writing", with a plain list of what is sent (job text and the section being edited, never the full profile or email). Off by default. Answers the local model's quality gap without giving up the privacy story.

### 14. Public job lists (M)

- **Simplify:** GitHub lists such as Summer 2027 internships, about 48K stars, feed its brand.
- **CeeVee, better and simpler:** publish a small number of high-quality lists in areas we serve, generated from the same feed as item 2, each ending with a link to CeeVee. Marketing task more than a product feature; schedule after the feed is reliable.

### 15. Mobile companion (Deferred, L)

- **Decision (Howard, 2026-11-26):** not now. Revisit after launch.

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
| Account menu and tabbed settings modal | 1 (Billing), 5 (Browser), 8 (Data), 12 (Connections), 13 (AI) |
| Network entry in left nav | 12 (People, Companies, Pipeline) |
| Dense lists and toggles | 2, 10, 12 |
| Light and dark themes | all |

Build the new components in turtlebox-ui once and reuse them in CeeVee and Golden: the checklist card (item 3), the record viewer (item 4) and the level slider (item 10) are likely candidates.

## Measures of success

- First search shows at least 20 relevant jobs within 60 seconds for the top 10 job types.
- Blank required fields at submit: zero.
- Autofill success on Greenhouse, Lever, Ashby, Workday: measured in CI against saved forms, with a published floor in evals/thresholds.json.
- Time from install to first filled application: under 10 minutes.
- Network: a LinkedIn export imports and merges in under a minute; every saved job with a known contact shows "people you know here"; follow-ups the user marks done each week.
- Trial to paid conversion and refund requests: refund requests should stay below 2%.

## Open questions for Howard

1. ~~After the 14-day trial, should autofill stay free?~~ Decided 2026-11-26: keep 14 days, fully locked, read-only after.
2. ~~Companion browser extension in v1 or v1.1?~~ Decided 2026-11-26: in v1.
3. ~~How to connect an optional stronger online model?~~ Decided 2026-11-26: neither for now (item 13 deferred).
4. ~~Mobile companion?~~ Decided 2026-11-26: not now, revisit after launch (item 15 deferred).
5. Network builder: is the full five-step scope wanted in v1, or only 12a (contacts, imports, people-at-company) with the rest in v1.1? Should calendar connection be limited to Google and Outlook at first?
