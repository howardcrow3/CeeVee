# CeeVee

A local AI app that helps you manage your job search. It runs on your computer: your resume, jobs and documents stay there.

**Download:** turtlebox.studio/ceevee (Windows 11, Apple silicon Mac). 14-day free trial, then $19.99 once for 2 computers.

Customer docs: [help](docs/customer/HELP.md), [privacy notice](docs/customer/PRIVACY.md), [terms](docs/customer/TERMS.md). Release steps: [docs/RELEASE_CHECKLIST.md](docs/RELEASE_CHECKLIST.md). Plans: [docs/PLAN.md](docs/PLAN.md), [docs/TECHNICAL_PLAN.md](docs/TECHNICAL_PLAN.md).

## Develop

Requirements: Rust (stable), Node 22, pnpm 10. Windows 11 x64 and macOS 13+ arm64 are the supported targets.

```sh
cd apps/desktop && pnpm install
pnpm tauri dev          # run the desktop app (from apps/desktop)
cargo test --workspace  # Rust tests
(cd ui && pnpm lint && pnpm check && pnpm test)   # UI checks
cargo deny check && (cd ui && pnpm license-check) # license allowlist
```

Layout: `apps/desktop` (Tauri shell and SvelteKit UI), `crates/*` (one crate per area), `docs/`.
