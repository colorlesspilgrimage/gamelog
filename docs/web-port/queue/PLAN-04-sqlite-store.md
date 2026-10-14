# PLAN: 04-sqlite-store

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice stores the library in SQLite on OPFS. Reload shows the same rows.

Non-goals:

- Do not open SQLite on the main thread.
- Do not use the `opfs` VFS.
- Do not set COOP or COEP headers.
- Do not add edit buttons.

## 2. Repo context

ASSUMPTION (unmerged 03-web-shell): `web/` shows the sample list and calls the search worker.

SETUP: same as slice 03, plus package `@sqlite.org/sqlite-wasm` `3.53.4-build2`.

FAST TEST: `cd web && npm test && npm run test:e2e`

FULL TEST: cargo tests, wasm-pack node tests, web unit tests, web e2e. CI-only: `packages`.

LINT: `cd web && npm run check && npm run lint && npm run format:check`.

COVERAGE: 85 percent via the slice 01 script on the git diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node on PATH must be `>=22.17`.
- `box:` not used.

## 3. Implementation steps

1. Add `web/src/lib/db/schema.ts`. Create tables if absent:

```sql
CREATE TABLE entries (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  system TEXT NOT NULL,
  status TEXT NOT NULL,
  hours REAL NOT NULL,
  rating INTEGER,
  release_date TEXT,
  last_played TEXT,
  notes TEXT NOT NULL DEFAULT '',
  cover_art TEXT
);
CREATE TABLE settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  sort_key TEXT NOT NULL,
  sort_reversed INTEGER NOT NULL
);
```

2. Add `web/src/lib/db/worker.ts`. This is the only SQLite connection. On `open`, call `installOpfsSAHPoolVfs` with directory `gamelog-sah`. Then open `gamelog.sqlite3`. If install fails, post `{type:'locked', message:'This library is open in another tab.'}`.

3. Handle messages `open`, `loadEntries`, `putEntries`, `loadSettings`, and `saveSettings`. Each message runs in one `BEGIN IMMEDIATE` transaction on that connection. `putEntries` deletes no absent logic: it inserts or replaces the posted ids only. Select existing keys and write in the same transaction.

4. Use bound parameters for every value. Do not concatenate SQL strings.

5. Add `web/src/lib/db/client.ts`. The page talks only to this client.

6. Remove `web/src/lib/sample.ts` from the page path. On first open, insert the settings row `title` and reversed 0. The entry table starts empty.

7. After a successful open, call `navigator.storage.persist()`. If it returns false, set a status string `The browser did not grant persistent storage.` Continue.

8. Keep the search worker. Feed it rows from `loadEntries`. It must not open SQLite.

## 4. Structure

`schema.ts` holds SQL text. `worker.ts` holds the message loop. `client.ts` holds the main-thread queue. One queue prevents two messages from interleaving. There is no second writer.

Playwright may import `/src/lib/db/client.ts` only under `vite dev`. Do not add a production seed route.

## 5. Tests to write

`web/src/lib/db/sql.test.ts` uses a fake statement recorder. Expected: `putEntries` binds title and id. It does not build SQL with the title text.

`web/e2e/persist.spec.ts` on `vite dev`:

- Import the client. Put Hades, Celeste, and Bastion. Reload. Expected: three titles.
- Save sort key `hours` and reversed 1. Reload. Expected: the list label shows Hours and reverse.
- Open a second page. Expected text: `This library is open in another tab.` The first page still shows the rows.

Keys in this spec follow slice 03. `a` still does nothing.

## 6. Acceptance criteria

- [ ] Reload keeps entries and sort settings.
- [ ] The empty library shows the empty sentence.
- [ ] A second tab does not write.
- [ ] SQL values are bound parameters.

| Criterion | Test |
|---|---|
| Reload | `persist.spec.ts` |
| Second tab | `persist.spec.ts` |
| Bound SQL | `sql.test.ts` |
| Empty state | `persist.spec.ts` first visit |

## 7. Feature map entry

```markdown
## Browser library store

The web app stores entries in SQLite on OPFS.

Entry point: `web/src/lib/db/worker.ts`.

Use steps:
1. Open the dev URL.
2. Insert rows through the database client in a test.
3. Reload the page.

Check: `cd web && npm run test:e2e -- e2e/persist.spec.ts`
Expected: the command exits 0.

Code: `web/src/lib/db/`
Tests: `web/e2e/persist.spec.ts`
```

## 8. Manual check script

```sh
cd web && npm run dev -- --host 127.0.0.1 --port 5173
```

Open the URL. Expected: `No entries yet. Press 'a' to add one.` In the devtools console, import the client module and put one entry titled `Hades` on system `PC`. Reload. Expected: the Hades row. Open a second tab. Expected: the lock sentence.

Do not pass the title through a shell. Use the console.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 03-web-shell): the search worker and list UI exist.
- Checked: package `@sqlite.org/sqlite-wasm` `3.53.4-build2`.
- Checked: SAH pool does not need COOP or COEP. Source: SQLite wasm persistence doc, OPFS SyncAccessHandle Pool section.
- The npm readme `OpfsDb` example does need those headers. Do not copy that example.
- UNCHECKED: `navigator.storage.persist()` prompt behavior. The test must not require a prompt.
- OPFS needs a secure context. `http://127.0.0.1` is a secure context. A bare file URL is not.
