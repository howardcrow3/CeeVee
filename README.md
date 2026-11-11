# CeeVee

A local AI app that helps you manage your job search. See [docs/PLAN.md](docs/PLAN.md) and [docs/TECHNICAL_PLAN.md](docs/TECHNICAL_PLAN.md).

## Develop

Requirements: Rust (stable), Node 22, pnpm 10. Windows 11 x64 and macOS 13+ arm64 are the supported targets.

```sh
cd apps/desktop/ui && pnpm install
pnpm tauri dev          # run the desktop app
cargo test --workspace  # Rust tests
pnpm lint && pnpm check && pnpm test   # UI checks
cargo deny check && pnpm license-check # license allowlist
```

Layout: `apps/desktop` (Tauri shell and SvelteKit UI), `crates/*` (one crate per area), `docs/`.
