# PLAN: 07-play-session

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice adds one play timer. Stop adds elapsed hours and sets last played.

Non-goals:

- Do not run two timers.
- Do not read the wall clock inside tests.
- Do not close the tab on `q`.

## 2. Repo context

ASSUMPTION (unmerged 06-notes-rating-delete): rows can be updated and deleted.

SETUP: same as slice 04.

FAST TEST: `cd web && npm test -- session && npm run test:e2e -- e2e/session.spec.ts`

FULL TEST: cargo, wasm, web unit, web e2e. CI-only: `packages`.

LINT: web check, lint, and format check.

COVERAGE: 85 percent on the diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node `>=22.17`.
- `box:` not used.

## 3. Implementation steps

1. Add `web/src/lib/session.ts`. A session stores `entryId` and `startedAtMs`. `elapsedMs(nowMs)` subtracts those numbers. `addedHours` divides milliseconds by `3600000`.

2. The footer shows `HH:MM:SS` while a session runs, including on another selected row. Format matches `format_duration` in `src/ui.rs`.

3. On stop, add hours, set `last_played` to the stop instant in UTC, and save in one `updateEntry` transaction. Status text: `Session ended: +<hours:.2>h to <title> (total <hours:.1>h).`

4. On `pagehide`, call the same stop path. Tests dispatch `pagehide`. They do not wait on a real clock.

5. If delete confirms the timed row, drop the session and do not add hours. That matches `confirm_delete`.

## 4. Structure

`session.ts` is pure. The Svelte footer only renders its result. The clock input is `nowMs` from the page, or from the test.

## 5. Tests to write

Unit tests in `web/src/lib/session.test.ts`:

- Start 0, now 3661000. Display `01:01:01`. Added hours are `3661000 / 3600000`.
- A second start while one runs does not replace it.

`web/e2e/session.spec.ts` stubs `nowMs` with `page.addInitScript` and a test hook `window.__gamelogSetNow` that exists only when `import.meta.env.DEV` is true. Vite must drop that hook from production. Assert the production bundle does not contain `__gamelogSetNow`.

| Key | Result |
|---|---|
| `t` on a row | Starts the timer. The footer names that title. |
| `t` on the same row | Stops, adds hours, sets last played, shows the status sentence. |
| `t` on a different row | Status contains `already running`. The first timer stays. |
| `j` and `k` during a session | Change selection. The footer stays. |
| `d` then `y` on the timed row | Removes the row and the timer. Hours of other rows stay. |
| `q` | Does nothing. |

Move the clock forward in the stub. Expected hours increase by the stub delta only.

## 6. Acceptance criteria

- [ ] One session can run.
- [ ] Stop updates hours and last played.
- [ ] `pagehide` saves the same way.
- [ ] Tests do not call the real clock.

| Criterion | Test |
|---|---|
| Elapsed format | `session.test.ts` |
| Start and stop | `session.spec.ts` |
| Second game blocked | `session.spec.ts` |
| Delete drops timer | `session.spec.ts` |
| pagehide | `session.spec.ts` |

## 7. Feature map entry

```markdown
## Web play session

The `t` key starts one timer and adds the elapsed hours when it stops.

Entry point: `web/src/lib/session.ts`.

Use steps:
1. Select a game. Press `t`.
2. Press `t` again.
3. Read the added hours in the status line.

Check: `cd web && npm run test:e2e -- e2e/session.spec.ts`
Expected: the command exits 0.

Code: `web/src/lib/session.ts`
Tests: `web/src/lib/session.test.ts`, `web/e2e/session.spec.ts`
```

## 8. Manual check script

Use the dev server. Select Hades. Press `t`. Expected footer contains `Hades` and a running clock. Press `j`. Expected: the footer still names Hades. Press `k`. Press `t`. Expected status starts with `Session ended:`.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 06-notes-rating-delete): delete confirmation exists.
- The dev clock hook must not ship. The bundle assertion guards that.
- `pagehide` save and the worker transaction are one message. A second message must not overlap it.
