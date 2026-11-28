# UI design rules

Source of truth: the Turtlebox UX framework, repo https://github.com/howardcrow3/turtlebox-ui (`docs/DESIGN.md`, `docs/CHARTS.md`, `tokens/tokens.json`). It replaces the older marketing-site design system. This page is a short digest so the rules are reviewable next to the code. Where the two differ, turtlebox-ui wins. Conflicts with the plans are tracked in `QUESTIONS.md`.

CeeVee vendors the tokens as `apps/desktop/ui/src/styles/turtlebox-tokens.css` (generated from turtlebox-ui `css/tokens.css`; do not edit by hand) and `static/brand/tokens.json`.

## Rules to build against

- **Chat anchors the app.** Home is a conversation. Work areas (Job types, Jobs, Applications, Inbox, Documents, Network) sit in the left sidebar. Settings live in a modal with tabs on the left, opened from the account menu at the bottom left.
- **Warm neutrals and one gold accent.** Gold is a fill or underline, never text. `danger` is the only added hue.
- **Role tokens, never hex.** Components use `paper`, `surface`, `line`, `ink`, `body`, `border-control`, `accent`, `on-ink`, `on-accent`, `danger`, `focus-ring`, plus `sidebar`, `hover`, `selected`, `muted`, `accent-soft`, `ok`, `scrim`. Light and dark are both first class and follow `prefers-color-scheme`; `data-theme` forces one.
- **Type.** Inter only, no serif. 13px body, 12.5px buttons, 15px dialog titles. Fonts are self-hosted woff2 (400, 500, 600).
- **Density.** Button 26px (small 22px), input 28px, select 26px, chip 22px, nav row 28px, table row 30px. Toggles, segmented controls and selects instead of checkboxes and long forms.
- **Shape.** Radii 6, 10 and 14px. Borders carry structure; one soft shadow is allowed on menus and dialogs.
- **Focus** is a visible `focus-ring` outline. **Motion** is short and disappears under `prefers-reduced-motion`.
- **Voice.** Plain, specific, unhurried. No exclamation marks or hype words. Errors say what went wrong and what to do. Statuses are literal.

## Rollout

CeeVee adopts the framework in small PRs following the turtlebox-ui adoption plan: tokens and base CSS, core controls, app shell and account menu, settings modal, Job types, lists, three-pane chat, chat home, Sign in with Google, then retire the old header and replaced pages.
