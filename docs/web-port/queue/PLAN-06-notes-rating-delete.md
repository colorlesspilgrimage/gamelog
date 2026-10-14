# PLAN: 06-notes-rating-delete

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice adds notes, the 0 to 5 rating, and confirmed delete.

Non-goals:

- Do not add the session timer.
- Do not import files.
- Do not remove cover bytes here. There are none yet.

## 2. Repo context

ASSUMPTION (unmerged 05-edit-fields): add and field edit keys work and persist.

SETUP: same as slice 04.

FAST TEST: `cd web && npm run test:e2e -- e2e/notes-rating-delete.spec.ts`

FULL TEST: cargo, wasm, web unit, web e2e. CI-only: `packages`.

LINT: web check, lint, and format check.

COVERAGE: 85 percent on the diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node `>=22.17`.
- `box:` not used.

## 3. Implementation steps

1. Add `web/src/lib/ui/NotesEditor.svelte`. Enter inserts `\n`. Esc saves. The notes column stores the text as typed, including spaces.

2. Add `web/src/lib/ui/RatingDialog.svelte`. Show the current stars or `Unrated`.

3. Add `web/src/lib/ui/DeleteDialog.svelte`. Copy: `Delete '<title>'?` and `This cannot be undone.`

4. Add `deleteEntry` on the database worker. Delete the row inside `BEGIN IMMEDIATE`.

5. After delete, clamp the selection the way `App::confirm_delete` does.

## 4. Structure

Each dialog is its own file. The page switches mode. List keys do not run while a dialog is open, except the keys named in section 5.

## 5. Tests to write

`web/e2e/notes-rating-delete.spec.ts` with three seeded rows.

| Key | Result |
|---|---|
| Enter in normal mode | Opens notes for the selected row. |
| Enter inside notes | Inserts a newline. It does not close the editor. |
| Esc inside notes | Saves and returns to the list. |
| `r` | Opens rating. |
| `0` through `5` | Saves that star count and closes. |
| `6` through `9` | Do nothing. |
| `u` or Backspace | Sets rating to null and closes. |
| Esc in rating | Closes and keeps the old rating. |
| `d` | Opens confirm. |
| `y` | Deletes the row. |
| `n` or Esc | Cancels. The row stays. |
| Any other key in confirm | Does nothing. |
| `j` while notes are open | Types `j` if it is a character input. It does not move the list. |

Reload after notes and rating. Expected: the same text and stars.

## 6. Acceptance criteria

- [ ] Notes keep newlines after reload.
- [ ] Rating accepts only 0 through 5, or clear.
- [ ] Delete requires `y`.
- [ ] Esc in rating and delete does not write.

| Criterion | Test |
|---|---|
| Notes | `notes-rating-delete.spec.ts` |
| Rating | `notes-rating-delete.spec.ts` |
| Delete | `notes-rating-delete.spec.ts` |

## 7. Feature map entry

```markdown
## Web notes, rating, and delete

The web app edits notes, sets a 0 to 5 rating, and deletes after confirmation.

Entry point: notes key Enter, rating key `r`, delete key `d`.

Use steps:
1. Select a row. Press Enter. Type a note. Press Esc.
2. Press `r`, then `5`.
3. Press `d`, then `y`, to delete.

Check: `cd web && npm run test:e2e -- e2e/notes-rating-delete.spec.ts`
Expected: the command exits 0.

Code: `web/src/lib/ui/NotesEditor.svelte`, `web/src/lib/ui/RatingDialog.svelte`, `web/src/lib/ui/DeleteDialog.svelte`
Tests: `web/e2e/notes-rating-delete.spec.ts`
```

## 8. Manual check script

Seed three rows. Select Celeste. Press Enter. Type `Line one`. Press Enter. Type `Line two`. Press Esc. Press `r`, then `4`. Reload. Expected: both lines and four stars. Press `d`, then `n`. Expected: Celeste remains. Press `d`, then `y`. Expected: Celeste is gone.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 05-edit-fields): the dialog focus trap exists.
- Render notes as text. Do not use HTML interpolation. This avoids CWE-79.
- Delete and the following list read share one transaction so the UI cannot show a removed id.
