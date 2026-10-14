# PLAN: web-port

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This document is the queue overview. Each slice has its own plan. The work is a plan only. Do not implement it here.

Non-goals:

- Do not remove the terminal app.
- Do not change `.deb`, `.rpm`, or AUR install paths.
- Do not change library JSON version 1.
- Do not add accounts, sync, or a hosted database.
- Do not call SteamGridDB or RAWG from the static web app.
- Do not add a proxy that sees API keys.
- Do not store cover-source API keys in the web app.
- Do not close the browser tab from `q` or Esc.

## 2. Repo context

Exploration ran on `master`. Branch `feat/web-port` does not exist. `docs/FEATURES.md` does not exist. There is no repo lessons file.

The terminal app is one Cargo package, `gamelog`, edition 2024. Entry is `src/main.rs`. Run it with `cargo run`. Data is `~/.gamelog/entries.json`, `covers/`, `settings.toml`, and `keybindings.toml` (`src/paths.rs`). Saves of `entries.json` use a temp file and `rename` (`Library::save` in `src/storage.rs`).

Library JSON is version 1. `LibraryFile` has `version` and `entries`. A real serde sample, built from the same field types and versions as `Cargo.toml`, is:

```json
{
  "version": 1,
  "entries": [
    {
      "id": "5ad71341-6fc2-48ba-9041-23ca4bff0af4",
      "title": "Hades",
      "release_date": "2020-09-17",
      "system": "PC",
      "status": "Played",
      "hours": 42.5,
      "rating": 5,
      "cover_art": "covers/5ad71341-6fc2-48ba-9041-23ca4bff0af4.jpg",
      "notes": "Line one\nLine two",
      "last_played": "2025-09-16T05:20:00.123Z"
    }
  ]
}
```

`status` is `WantToPlay`, `Playing`, or `Played`. `rating` is a number from 0 to 5, or null. `cover_art` is a relative path string, or null. `notes` defaults to `""`. `last_played` defaults to null. An old object without `last_played` still loads. `src/entry.rs` tests that case.

Cover files live under `~/.gamelog/covers/<id>.<ext>`. Fetch uses ureq in `src/cover_fetch.rs`. SteamGridDB uses a bearer key. RAWG puts the key in the query. A miss opens `CoverFetchChoice`. Bulk fetch uses 4 workers and retries HTTP 429.

CSV rules live in `src/csv_io.rs`. Export columns are title, system, status, hours, rating, release_date, last_played, notes. Import needs `title` or `name`, and `system` or `platform`. Duplicate title plus system is skipped in `App::import_csv`. The match ignores case.

CI is `.github/workflows/ci.yml`. Job `test` runs `cargo build --locked` and `cargo test --locked`. Job `packages` builds and installs `.deb` and `.rpm`. That job is CI-only for a normal dev machine. Release workflow is `.github/workflows/release.yml`.

SETUP: Install Rust stable 1.85 or newer. The manifest uses edition 2024. Then run `cargo fetch --locked`. Web slices also need Node 22.17 or newer, `cd web && npm ci`, and `cd web && npx playwright install --with-deps chromium`.

FAST TEST: `cargo test --locked`

FULL TEST: `cargo test --locked`. After the web slices, also run `cd web && npm test` and `cd web && npm run test:e2e`. CI-only: the `packages` job.

LINT: none. CI does not run rustfmt or clippy.

COVERAGE: none in CI today. Slice 01 adds `script/check-file-coverage.py`. Each later slice runs it on `git diff --name-only master...HEAD`. The gate is 85 percent line coverage per changed production file.

SHARED DEPENDENCY DIRECTORIES: none on `master`. After the slices, `target/` and `web/node_modules/` are shared.

Environment facts:

- `cloud-agent VM:` default `cargo` is 1.83.0. It rejects edition 2024. `rustup toolchain install stable --profile minimal` installed rustc 1.99.0 (b940084d7, 2026-09-28). `cargo +stable test --locked` passed.
- `cloud-agent VM:` default `node` is v22.14.0 at `/exec-daemon/node`. SvelteKit 3 asks for Node `>=22.17`. Node v22.23.3 is at `/home/ubuntu/.nvm/versions/node/v22.23.3/bin/node`.
- `box:` this run did not use the shared box.

## 3. Implementation steps

The queue is the implementation order. Do not start a slice before its dependencies.

1. `01-core-crate`. Move pure library rules into `crates/gamelog-core`. The terminal binary stays at the repo root. Behavior of the terminal app stays the same. Depends on nothing.

2. `02-wasm-api`. Export those rules through wasm-bindgen. Test them with `wasm-pack test --node`. Depends on `01-core-crate`.

3. `03-web-shell`. Add the SvelteKit app, the split pane, and the list keymap. Search and sort run in a worker that calls the wasm module. Data is a fixed sample. Depends on `02-wasm-api`.

4. `04-sqlite-store`. Replace the sample with SQLite in one OPFS worker. Reload keeps the rows. Depends on `03-web-shell`.

5. `05-edit-fields`. Add and edit title, system, status, hours, release date, and last played. Depends on `04-sqlite-store`.

6. `06-notes-rating-delete`. Edit notes, set a rating, and delete with confirmation. Depends on `05-edit-fields`.

7. `07-play-session`. Add the one-session timer. A page hide saves it. Depends on `06-notes-rating-delete`.

8. `08-import-export`. Import version-1 JSON and CSV in a worker. Export CSV. Depends on `04-sqlite-store` and `03-web-shell`.

9. `09-cover-images`. Import a local image. Store bytes in SQLite. Show a lazy responsive image. Depends on `06-notes-rating-delete`.

10. `10-offline-pwa`. Precache the app shell. Add the install manifest. Depends on `03-web-shell` and `04-sqlite-store`.

11. `11-keybindings`. Store key changes in SQLite. The help bar reads them. Depends on `10-offline-pwa` so every action already exists.

## 4. Structure

Keep the terminal package at the repository root. `cargo install --path .` and `packaging/aur/PKGBUILD` expect that. `target/release/gamelog` stays the binary path in `Cargo.toml` package metadata.

Add a workspace in the root `Cargo.toml`. Members are `.` and `crates/gamelog-core`. Set `resolver = "3"`. Put the web app in `web/`. Do not make `web/` a Cargo member.

`crates/gamelog-core` holds parse, sort, search match, CSV, and duplicate rules. `web/src/lib/workers/` holds the search worker, the import worker, and the database worker. `web/src/routes/` holds one page. UI components go in `web/src/lib/ui/`. One feature uses one file. Keep each new file under 400 lines. Keep each new function under 50 lines.

## 5. Tests to write

Slice 01 moves the current 19 unit tests and adds property tests for sort. Slice 02 adds node tests for the wasm exports. Slices 03 through 11 add Vitest tests and Playwright keyboard tests. Keyboard tests use at least three rows. They send keys with `page.keyboard`. They do not use `focus()` as a substitute for a key.

Do not use the network, the wall clock, or unseeded random values. Inject timestamps for the session timer. Seed property tests through proptest.

## 6. Acceptance criteria

- [ ] The terminal binary still builds at `target/release/gamelog`.
- [ ] `cargo test --locked` passes.
- [ ] The web app loads on localhost and shows the library.
- [ ] List, filter, search, and sort follow the terminal rules.
- [ ] Add, edit, notes, rating, delete, and the session timer match the terminal rules.
- [ ] A version-1 `entries.json` file imports, including an old entry with no `last_played`.
- [ ] CSV import and export follow `src/csv_io.rs`.
- [ ] A chosen image displays, lazy and sized. AVIF is used only when the browser encodes it.
- [ ] Reload keeps the library. Offline reload after one online load shows it.
- [ ] `q` and a bare Esc do not close the tab.
- [ ] The web app does not request SteamGridDB or RAWG.
- [ ] A second tab shows a lock message and does not write.

| Criterion | Test |
|---|---|
| Terminal tests stay green | `cargo test --locked` in slice 01 |
| Wasm rules match Rust | `wasm-pack test --node` in slice 02 |
| List keymap | Playwright in slice 03 |
| Reload keeps rows | Playwright in slice 04 |
| Field edits | Playwright in slice 05 |
| Notes, rating, delete | Playwright in slice 06 |
| Session timer | Playwright in slice 07 |
| JSON and CSV | Playwright in slice 08 |
| Cover image | Playwright in slice 09 |
| Offline shell | Playwright in slice 10 |
| Custom keys | Playwright in slice 11 |

## 7. Feature map entry

Slice 01 creates `docs/FEATURES.md`. Later slices append. No slice deletes another entry. The first terminal entry is in `PLAN-01-core-crate.md`.

## 8. Manual check script

After every slice, run that slice's script. After the whole queue, run:

```sh
cargo test --locked
cargo build --release --locked
test -x target/release/gamelog
cd web && npm test && npm run test:e2e && npm run build
```

Expected: both test commands report no failures. `target/release/gamelog` exists. `web/build/index.html` exists.

Then open the preview URL from slice 10. Import the sample JSON from section 2. Expected: one row, title `Hades`, system `PC`, status Played, hours `42.5`, five stars. Reload the page. Expected: the same row. Stop the server. Run the offline check in slice 10.

## 9. Risks and open assumptions

The terminal app stays. Commit `53b3509` removed an older GUI and left the terminal product. Releases, CI package installs, and `~/.gamelog` belong to that binary. The web app is a second client. It cannot read `~/.gamelog` by itself. JSON import is the bridge.

Compile this logic to wasm. It is pure and already tested:

- Status cycle, rating clamp, title search, and `SortKey::compare` in `src/entry.rs`.
- CSV read and write in `src/csv_io.rs`.
- Version-1 JSON parse and write. The shape is `LibraryFile` in `src/storage.rs`.
- Duplicate title-plus-system skip. That block is in `App::import_csv`.
- Empty title, empty system, hours, date, and rating checks. Those checks are in `src/app.rs`.

Leave this code in the terminal crate. It does not carry portable logic:

- `src/ui.rs` draws ratatui widgets.
- `src/main.rs` reads crossterm keys.
- `src/keybindings.rs` stores crossterm `KeyCode`. The web map is TypeScript with the same default keys.
- `src/paths.rs` resolves the home directory.
- `src/storage.rs` does filesystem rename. The web store is SQLite.
- `src/cover_fetch.rs` calls ureq. ureq does not run in wasm.
- `src/app.rs` owns threads, the image cache, and `ListState`.

Wasm exports must not call `chrono::Local`. They take an east-of-UTC offset in seconds. The terminal app still uses `Local`.

Stack decisions:

- UI: SvelteKit `3.0.1`, Svelte `5.57.2`, Vite `8.3.4`, `@sveltejs/vite-plugin-svelte` `7.3.1`. Config goes in `web/vite.config.js`. Kit 3 does not read `svelte.config.js`. Source: https://svelte.dev/docs/kit/adapter-static on 2026-10-14.
- TypeScript `6.0.3`. The Kit peer range is `^6`. Do not install TypeScript 7.
- Adapter: `@sveltejs/adapter-static` `4.0.0`. Set `fallback: 'index.html'`. The app has one route and does not prerender it. Set `export const ssr = false` in the root layout. Source: https://svelte.dev/docs/kit/single-page-apps on 2026-10-14.
- Service worker: `web/src/service-worker/index.ts`. Kit bundles and registers that file. Source: https://svelte.dev/docs/kit/service-workers on 2026-10-14.
- Database: `@sqlite.org/sqlite-wasm` `3.53.4-build2` from `npm view` on 2026-10-14. Open it only in one dedicated worker. Call `installOpfsSAHPoolVfs`. Do not construct `OpfsDb`. The `opfs` VFS needs COOP and COEP. The SAH pool does not. Source: https://sqlite.org/wasm/doc/trunk/persistence.md on 2026-10-14.
- Wasm tools: `wasm-bindgen` `0.2.129` (`cargo info`). `wasm-pack` `0.15.0` (GitHub release `v0.15.0`). On wasm32, enable `uuid` feature `js` and `getrandom` `0.4.3` feature `wasm_js`.
- Images: store the original bytes. Make a thumb and a detail derivative. Use `image/avif` only when `blob.type` is `image/avif`. Otherwise use webp, then jpeg. MDN says an unknown `toBlob` type becomes png. `<img>` uses `srcset`, width, height, and `loading="lazy"`.
- E2E image for CI visuals, if a later job adds them: `mcr.microsoft.com/playwright:v1.64.1-noble`. The tag exists on MCR. `@playwright/test` is `1.64.1`. Do not update snapshots on a dev machine.

Checked network limit: SteamGridDB OPTIONS with `Origin: https://example.com` and `Access-Control-Request-Headers: authorization` returned HTTP 200 and `allow: GET`. It did not return `Access-Control-Allow-Origin`. A browser preflight for the bearer call fails. RAWG OPTIONS did return `access-control-allow-origin: *`. RAWG GET without a key returned HTTP 401 and no `Access-Control-Allow-Origin` header. UNCHECKED: a RAWG HTTP 200 body response. Do not use either API from the web app.

UNCHECKED, and not an acceptance criterion:

- Whether `vite preview` maps `/` to the static fallback.
- The exact CSP token list the browser needs for this wasm build.
- Whether `navigator.storage.persist()` shows a prompt.
- Which browsers return `blob.type === 'image/avif'`.

`cargo +stable fmt --check` fails on `master` in `src/app.rs`, `src/entry.rs`, `src/storage.rs`, and `src/ui.rs`. `cargo +stable clippy --locked --all-targets -- -D warnings` fails on unused `Rating::MIN` and on `CoverSource`'s `Default`. Do not add a repo-wide clippy gate in this queue.
