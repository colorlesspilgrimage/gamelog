# PLAN: 05-edit-fields

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice edits title, system, status, hours, release date, and last played. It can add a row.

Non-goals:

- Do not edit notes here.
- Do not set a rating here.
- Do not delete here.
- Do not start a play session.

## 2. Repo context

ASSUMPTION (unmerged 04-sqlite-store): the database worker accepts `putEntries`, `loadEntries`, and settings messages.
ASSUMPTION (unmerged 02-wasm-api): `validate_title`, `validate_system`, `validate_hours`, `validate_release_date`, and `set_last_played` exist.

SETUP: same as slice 04.

FAST TEST: `cd web && npm test && npm run test:e2e -- e2e/edit-fields.spec.ts`

FULL TEST: full cargo, wasm, unit, and e2e suites. CI-only: `packages`.

LINT: web check, lint, and format check.

COVERAGE: 85 percent on the diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node `>=22.17`.
- `box:` not used.

## 3. Implementation steps

1. Add message `updateEntry` in `web/src/lib/db/entries.ts`. The worker calls it inside `BEGIN IMMEDIATE`. Read the row and write the row in that transaction.

2. Add `web/src/lib/ui/TextDialog.svelte`. One dialog, many labels. Focus the input on open. Trap Tab inside the dialog.

3. Wire keys in `web/src/lib/keys/edit.ts`. The page offset for last played is `-new Date().getTimezoneOffset() * 60`. Pass that offset to wasm. Do not parse dates in the component.

4. On a successful add, clear a search that hides the new id. If the system filter hides it, set the filter to all. Then select the new id.

5. On a successful edit, keep the same id selected when it is still visible. If it is hidden, select index 0.

6. Persist with `updateEntry` before you close the dialog.

## 4. Structure

Dialog copy matches the terminal prompts. New title prompt: `Enter the game's title:`. New system prompt: `Enter the system/platform:`. Hours prompt: `Enter hours played:`. Date prompt: `Enter date as YYYY-MM-DD (blank to clear):`.

New files stay under 400 lines. Do not add this UI to a Rust file.

## 5. Tests to write

`web/e2e/edit-fields.spec.ts` seeds three rows through the dev client, clicks the app root, then uses `page.keyboard`.

| Key | Result |
|---|---|
| `a`, then title, Enter, system, Enter | Creates a Want to Play row with 0 hours and no rating. |
| Enter on an empty title | Status `Title cannot be empty.` The dialog stays. |
| Enter on an empty system | Status `System cannot be empty.` |
| Esc during add | No new row. |
| `T` | Edits the title. Empty trim is rejected. |
| `p` | Edits the system. |
| `s` | Cycles Want to Play, Playing, Played, Want to Play. |
| `h` | Sets hours. `-1` shows `Enter a number >= 0.` |
| `R` | Sets release date. `2020-13-01` shows `Use YYYY-MM-DD format.` Blank clears it. |
| `L` | Sets last played. The same civil date keeps the old timestamp. Blank clears it. |
| `/`, `j` | `j` types into search. It does not move. |
| Arrow keys inside a dialog | Do nothing to the text cursor and nothing to the list. |
| `d`, `r`, Enter in normal mode, `t` | Do nothing. |

Seed last played `2025-06-01T15:00:00Z` and offset 0. Open `L`. Submit `2025-06-01`. Expected stored instant: `2025-06-01T15:00:00Z`.

Add a row whose title is hidden by query `zzz`. Expected: the query clears and the new row is selected.

## 6. Acceptance criteria

- [ ] Add and the six edits persist across reload.
- [ ] Invalid input leaves the old value.
- [ ] The selected row follows the terminal reveal rules.
- [ ] Dialog keys do not change the list.

| Criterion | Test |
|---|---|
| Add and validation | `edit-fields.spec.ts` |
| Status and hours | `edit-fields.spec.ts` |
| Dates | `edit-fields.spec.ts` |
| Reveal | `edit-fields.spec.ts` |

## 7. Feature map entry

```markdown
## Web field edits

The web app adds a game and edits its title, system, status, hours, and dates.

Entry point: `web/src/lib/keys/edit.ts`.

Use steps:
1. Press `a`. Type `Hades`. Press Enter. Type `PC`. Press Enter.
2. Press `s` to change status.
3. Reload the page.

Check: `cd web && npm run test:e2e -- e2e/edit-fields.spec.ts`
Expected: the command exits 0.

Code: `web/src/lib/ui/TextDialog.svelte`, `web/src/lib/db/entries.ts`
Tests: `web/e2e/edit-fields.spec.ts`
```

## 8. Manual check script

Start `npm run dev` as in slice 04. Press `a`. Type `Outer Wilds`. Press Enter. Type `PC`. Press Enter. Press `h`. Type `3.5`. Press Enter. Press `R`. Type `2019-05-30`. Press Enter. Reload. Expected: Outer Wilds, PC, 3.5 hours, release `2019-05-30`.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 04-sqlite-store): one database worker and `BEGIN IMMEDIATE` per message.
- ASSUMPTION (unmerged 02-wasm-api): validators and `set_last_played` exist.
- The JS offset sign is fixed by the slice 02 test. East seconds equal `-getTimezoneOffset() * 60`.
- Write the new row and the selection reload from the same worker transaction. Do not read the list from a second connection.
