# PLAN: 03-web-shell

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice adds the static web shell and the list keymap. Data is a fixed three-game sample.

Non-goals:

- Do not write a database.
- Do not add edit dialogs.
- Do not register a service worker yet.
- Do not fetch cover art.

## 2. Repo context

ASSUMPTION (unmerged 01-core-crate): workspace and coverage script exist.
ASSUMPTION (unmerged 02-wasm-api): `wasm-pack build --release --target web --out-dir pkg -- --features wasm` writes `crates/gamelog-core/pkg`.

SETUP: Node `>=22.17`. Rust stable. wasm-pack 0.15.0. Build the pkg above. Then `cd web && npm ci && npx playwright install --with-deps chromium`.

FAST TEST: `cd web && npm test` and `cd web && npm run test:e2e`.

FULL TEST: `cargo test --locked`, `wasm-pack test --node crates/gamelog-core`, `cd web && npm test`, `cd web && npm run test:e2e`. CI-only: `packages`.

LINT: `cd web && npm run check && npm run lint && npm run format:check`.

COVERAGE: Vitest v8 JSON plus `script/check-file-coverage.py --format vitest`. Gate 85 percent on changed production files from `git diff --name-only master...HEAD`.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` default Node v22.14.0 is too old for Kit 3. Use Node 22.23.3 or newer.
- `box:` not used.

## 3. Implementation steps

1. Create `web/` as a SvelteKit app. Pin `@sveltejs/kit` `3.0.1`, `svelte` `5.57.2`, `vite` `8.3.4`, `@sveltejs/vite-plugin-svelte` `7.3.1`, `@sveltejs/adapter-static` `4.0.0`, `typescript` `6.0.3`, `vitest` `5.0.4`, `@vitest/coverage-v8` `5.0.4`, `eslint` `10.12.0`, `eslint-plugin-svelte` `3.23.1`, `prettier` `3.9.10`, `eslint-config-prettier` `10.1.8`, `svelte-check` `4.7.6`, `@playwright/test` `1.64.1`.

2. Put the adapter in `web/vite.config.js`. Set `fallback: 'index.html'`. In `web/src/routes/+layout.ts`, export `ssr = false`. Do not add `svelte.config.js`.

3. Add `web/src/lib/sample.ts` with Hades (PC, Played, 42.5, rating 5), Celeste (Switch, Playing, 12, rating 4), and Bastion (PC, Want to Play, 0, no rating).

4. Add `web/src/lib/workers/search-worker.ts`. It imports the wasm pkg and calls `visible_ids`. The main thread posts entries, filter, query, sort key, and reversed.

5. Add `web/src/lib/ui/LibraryList.svelte` and `web/src/lib/ui/DetailPane.svelte`. The list window renders at most 30 rows plus a small overscan. The detail pane shows title, system, release, status, hours to one decimal, rating, and last played. Empty selection shows `No entries yet. Press 'a' to add one.`

6. Add `web/src/lib/keys/normal.ts`. Handle the keys in section 5. Call `preventDefault` only for handled keys that have no Ctrl or Meta modifier.

7. Add a help bar that shows the active keys. Add `data-app-root` on the shell.

8. Add scripts `check`, `lint`, `format:check`, `test`, and `test:e2e`.

9. Add a `web` job to CI. Use Node 22. Install wasm-pack. Build the pkg. Run `npm ci`, check, unit tests, and Playwright chromium. Do not run screenshot updates.

## 4. Structure

`web/src/routes/+page.svelte` composes the list and the detail. It does not own sort math. The worker owns `visible_ids`. Sample data is the only store in this slice. Slice 04 deletes that path.

Colors: page `#ffffff`, text `#1a1a1a`, selected row `#1a1a1a` with white text, played `#0d5c2e`, playing `#7a5b00`, backlog `#595959`. These pairs are for WCAG 2.2 AA body text. Add a visible focus ring.

Status glyphs match the terminal: `○`, `◐`, `●`.

## 5. Tests to write

Playwright file `web/e2e/list-keys.spec.ts`. Click `[data-app-root]` once. Then use only `page.keyboard`. Fixture is the three sample rows.

Normal mode:

| Key | Result |
|---|---|
| ArrowDown, `j` | Move down. Stop on the last row. |
| ArrowUp, `k` | Move up. Stop on the first row. |
| Tab | Filter cycle All, PC, Switch, All. Prevent focus leave. |
| Shift+Tab | Filter cycle All, Switch, PC, All. |
| `o` | Cycle sort Title, Hours, Rating, Release Date, Last Played, Status, then Title. Each `o` clears reverse. |
| `O` | Toggle reverse. Keep the selected id when it is still visible. |
| `/` | Open search. Typed letters change the query. `j` is a letter here, not a move. |
| ArrowUp, ArrowDown in search | Move the selection. |
| Enter in search | Keep the query and close search mode. |
| Esc in search | Clear the query and close search mode. |
| Backspace in search | Delete one code point. |
| Esc when the query is empty | Do nothing. |
| `a`, `d`, `T`, `p`, `R`, `L`, `s`, `h`, `t`, `r`, `c`, `F`, `I`, `E`, `q` | Do nothing. |
| `x` | Do nothing. |
| Ctrl+R | Do not handle it. |

Search `had` leaves Hades only. Search `   ` shows all three. A selected id that remains in the list stays selected.

Unit test the window function in `web/src/lib/ui/window.ts`. For 200 rows and a start index of 0, it returns at most 30 plus overscan.

## 6. Acceptance criteria

- [ ] The page shows three sample games.
- [ ] The keymap in section 5 holds.
- [ ] Search uses the worker. A slow worker still ends with the typed query.
- [ ] `npm run build` writes `web/build/index.html`.
- [ ] The terminal suite still passes.

| Criterion | Test |
|---|---|
| Navigation and filter | `list-keys.spec.ts` |
| Search letters | `list-keys.spec.ts` |
| Keys that do nothing | `list-keys.spec.ts` |
| Window size | `window.ts` unit test |
| Build output | manual check |

## 7. Feature map entry

```markdown
## Web library shell

A static page shows a sample library and the terminal list keys.

Entry point: `cd web && npm run dev`.

Use steps:
1. Open the dev URL.
2. Press `j` and `k` to move.
3. Press `/`, type `had`, and press Enter.

Check: `cd web && npm run test:e2e`
Expected: the command exits 0.

Code: `web/src/routes/+page.svelte`, `web/src/lib/workers/search-worker.ts`
Tests: `web/e2e/list-keys.spec.ts`
```

## 8. Manual check script

```sh
cd web && npm run dev -- --host 127.0.0.1 --port 5173
```

Open `http://127.0.0.1:5173`. Expected titles: Bastion, Celeste, Hades, in title order. Press `/`, type `cel`, press Enter. Expected: only Celeste. Press Esc. Expected: three rows. Press `q`. Expected: the page stays open.

Stop the server with Ctrl+C in its terminal. Do not put a title into the shell command.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 02-wasm-api): the pkg exports `visible_ids`.
- Checked: Kit 3 uses `vite.config.js`. Adapter-static 4.0.0 peers on Kit 3.
- UNCHECKED: whether `vite preview` serves `index.html` for `/`. Acceptance uses the build file and the dev server.
- CSP is deferred until a wasm response is visible in the browser. Do not set `connect-src` to any host but `'self'`.
