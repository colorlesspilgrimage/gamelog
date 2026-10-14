# PLAN: 08-import-export

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice imports a gamelog JSON file and CSV. It exports CSV.

Non-goals:

- Do not read `~/.gamelog` from disk.
- Do not download cover bytes from `cover_art` paths.
- Do not import `keybindings.toml`.
- Do not shell out with a game title.

## 2. Repo context

ASSUMPTION (unmerged 04-sqlite-store): `putEntries` and `loadEntries` exist.
ASSUMPTION (unmerged 02-wasm-api): `parse_library_json`, `import_csv_json`, `export_csv_json`, and `partition_new_json` exist.
ASSUMPTION (unmerged 03-web-shell): the list can show the result.

SETUP: same as slice 04.

FAST TEST: `cd web && npm test -- import && npm run test:e2e -- e2e/import-export.spec.ts`

FULL TEST: cargo, wasm, web unit, web e2e. CI-only: `packages`.

LINT: web check, lint, and format check.

COVERAGE: 85 percent on the diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node `>=22.17`.
- `box:` not used.

## 3. Implementation steps

1. Add `web/src/lib/workers/import-worker.ts`. It calls wasm. It does not open SQLite. The main thread sends the parsed rows to the database worker.

2. Reject a file larger than 20 MiB before parse. Show `File is too large.`

3. JSON import keeps ids. Reject a version other than 1. Skip an id that already exists. Also skip a title-plus-system duplicate. Keep the first row of a duplicate pair inside the file.

4. Accept `cover_art` only when it matches `^covers/[0-9a-fA-F-]{36}\.(png|jpg|jpeg|webp|gif|avif)$`. Any other value becomes null. Count those rows in the status line `Cover image is not in the JSON file.`

5. CSV import gives new ids through the wasm parser. It uses the same duplicate key as JSON. Bad rows report the count and the first line, as `App::import_csv` does.

6. Key `I` opens a file input. Key `E` builds CSV for every row, sorted by title, and downloads `gamelog-export.csv`. Export ignores the current filter.

7. The database message `importRows` runs in one `BEGIN IMMEDIATE`. It reads existing keys and inserts new rows before commit.

## 4. Structure

The worker file only parses. `web/src/lib/import/apply.ts` holds the size check, the cover path check, and the status sentence. Keep each file under 400 lines.

Write fixture files from test code. If a shell command must take a path, put `--` before the path. Reject a path that contains a newline or a single quote.

## 5. Tests to write

`web/src/lib/import/apply.test.ts`:

- Version 2 fails.
- The Hades sample from `PLAN.md` yields one row and a cover warning.
- An old entry with no `last_played` imports.
- `cover_art` `../secret` becomes null.
- A 20 MiB plus one byte file is rejected and wasm is not called.

`web/e2e/import-export.spec.ts` uses three existing rows and a generated file. Set the file on `input[type=file]` with Playwright. Do not type a path.

| Key | Result |
|---|---|
| `I` then a valid JSON file | New rows appear. Duplicate title and system do not double. |
| `I` then CSV `Platform,NAME` | Imports. Extra columns are ignored. |
| `I` then a bad rating row | Good rows import. Status names the first bad line. |
| `E` | Download name is `gamelog-export.csv`. Header is the eight terminal columns. Order is title A-Z. |
| `j` while the file dialog is not open | Moves the list. |
| `E` during search | Still exports the whole library. |

CSV fixture includes `Hades, "Director's" Cut` so quotes are real data. Build that fixture in the test process.

## 6. Acceptance criteria

- [ ] Version-1 JSON imports and survives reload.
- [ ] Old entries without `last_played` import.
- [ ] CSV duplicates and bad rows match the terminal report.
- [ ] Export is the whole library in title order.
- [ ] A bad cover path is not stored.

| Criterion | Test |
|---|---|
| JSON sample | `apply.test.ts` and e2e |
| Old entry | `apply.test.ts` |
| CSV | `import-export.spec.ts` |
| Export order | `import-export.spec.ts` |
| Size limit | `apply.test.ts` |

## 7. Feature map entry

```markdown
## Web JSON and CSV import

The web app imports version-1 gamelog JSON and CSV, and it downloads CSV.

Entry point: keys `I` and `E`.

Use steps:
1. Press `I` and choose an `entries.json` file.
2. Read the new titles in the list.
3. Press `E` and open the CSV download.

Check: `cd web && npm run test:e2e -- e2e/import-export.spec.ts`
Expected: the command exits 0.

Code: `web/src/lib/workers/import-worker.ts`, `web/src/lib/import/apply.ts`
Tests: `web/e2e/import-export.spec.ts`
```

## 8. Manual check script

Create `/tmp/gamelog-entries.json` with the sample in `PLAN.md` section 2. Start the dev server. Press `I` and choose that file. Expected: Hades, a cover warning, and no image. Reload. Expected: Hades remains. Press `E`. Expected file name `gamelog-export.csv`. The first data field is `Hades`.

The shell write uses a fixed path. Do not embed a title in the shell.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 02-wasm-api): parse exports exist.
- ASSUMPTION (unmerged 04-sqlite-store): one writer transaction exists.
- Cover bytes are absent from JSON. The warning is the parity note.
- Bound every inserted column. A title with a quote must not change the SQL text. CWE-89.
