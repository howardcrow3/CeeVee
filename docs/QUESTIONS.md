# Open questions for Howard

Answer in the thread or edit this file. Newest at the bottom; delete a question once it is answered and reflected in the plans.

1. ~~macOS CI job cancelled instantly.~~ Answered by Howard: the error was "The configuration 'macos-14-arm64-us-default' is not supported", so the `macos-14` image is unavailable, not a billing limit. Switched to `macos-latest` (Apple silicon). Needs a CI run to confirm.
2. **Table count.** The plan says 31 core tables; the schema in PR #4 has 37 (the plan's list adds up to 37). I will fix the plan text to 37 unless you want the schema trimmed.
3. **App icon.** The icon is a generated placeholder (teal square, white disc). Do you have a logo or brand colors for CeeVee and turtlebox.studio?
4. **SQLite advisory RUSTSEC-2026-0459.** `sqlx` (0.8 and 0.9) bundles SQLite older than 3.54, which has a local-file advisory. I added a documented ignore in `deny.toml`. CeeVee only opens its own database file, so I judge the risk low. OK to accept until sqlx updates?
5. **Branches for stacked PRs.** Each small PR needs its own branch. I am using `claude/project-thread-d7t7rp-<topic>`, stacked on the previous PR, and will retarget to `main` as each one merges. Tell me if you want a different scheme.
6. **Signing accounts.** Apple Developer Program and Azure Trusted Signing need identity checks that can take days to weeks. Have you started either? The release pipeline PR can be written without them, but cannot be tested.
7. **Battery and quiet hours.** The scheduler already honors "pause" and quiet hours (settings keys `background.paused_until`, `background.quiet_start`, `background.quiet_end`), and its rules skip work under 20% battery, but nothing reads the battery level yet. Reading it needs a small platform-specific piece (Windows power status, macOS IOKit). OK to add it with the tray-menu PR, or leave battery gating out of v1?
