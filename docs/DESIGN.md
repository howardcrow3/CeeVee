# UI design rules

Source of truth: the Turtlebox Design System artifact (https://claude.ai/artifact/Wmu4iwBcjEh7T7E6pdrcZY), also kept in the `turtlebox/design-system/` folder of the highturtle repo. This page is a short digest so the rules are reviewable next to the code. Where the two differ, the artifact wins. Conflicts with the plans are tracked in `QUESTIONS.md`.

## Rules to build against

- **Warm neutrals and one accent.** Surfaces are ivory, warm white or charcoal. Gold is the only accent: underlines, filled status dots, and the primary button's hover fill. Gold is never text. `danger` is the one added hue, for errors only.
- **Role tokens, never hex.** Write components against `paper`, `surface`, `line`, `ink`, `body`, `border-control`, `accent`, `on-ink`, `on-accent`, `danger`, `focus-ring`. Light and dark are both first class and follow `prefers-color-scheme`; `data-theme` forces one.
- **Borders, not shadows.** 1px `line` rules or a `surface` plate. No shadows, gradients or rounded corners (status dots are round).
- **Type.** Cormorant Garamond for headings and the wordmark. Inter for body, buttons, labels and forms. Labels and buttons are 11px uppercase with 0.3em tracking, written in sentence case. Running text is at most 34em wide, left aligned. Fonts are self-hosted woff2.
- **Space** is an 8px scale: 8, 16, 24, 32, 48, 64, 96.
- **Motion** is 300ms with one ease-out curve and disappears under `prefers-reduced-motion`. **Focus** is a 2px `focus-ring` outline at a 4px offset, always visible.
- **No icon set, no emoji, no imagery.** Plain text, plus the turtle mark.
- **Voice.** Plain, specific, unhurried. No exclamation marks or hype words. Errors say what went wrong and what to do, without apology. Statuses are literal.

## Not yet covered by the system

The system describes a marketing site: header, hero, app card, status chip, text field, button, footer. CeeVee also needs an app shell, lists and tables, dialogs, the approvals queue, a chat panel, a document canvas and a tray menu. Those are built from the same tokens and rules and then proposed back upstream.
