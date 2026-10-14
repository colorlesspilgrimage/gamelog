# PLAN: 02-wasm-api

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice exports the core rules to JavaScript. It does not add a page.

Non-goals:

- Do not bind ratatui, crossterm, ureq, or filesystem storage.
- Do not call `chrono::Local` from an export.
- Do not change terminal behavior.

## 2. Repo context

ASSUMPTION (unmerged 01-core-crate): `crates/gamelog-core` exists and the terminal app depends on it.

SETUP: Rust stable 1.85 or newer. `rustup target add wasm32-unknown-unknown`. `cargo install wasm-pack --version 0.15.0`. `cargo fetch --locked`.

FAST TEST: `cargo test --locked -p gamelog-core` and `wasm-pack test --node crates/gamelog-core`.

FULL TEST: `cargo test --locked` and the wasm-pack command. CI-only: the `packages` job.

LINT: none.

COVERAGE: `cargo llvm-cov --locked --json --output-path /tmp/cov.json`. Then `python3 script/check-file-coverage.py --files /tmp/cov-files.txt --report /tmp/cov.json --format llvm`. Build `/tmp/cov-files.txt` with `git diff --name-only master...HEAD`. Gate is 85 percent.

SHARED DEPENDENCY DIRECTORIES: `target/`.

Environment facts:

- `cloud-agent VM:` use `cargo +stable` when the default cargo is 1.83.0.
- `box:` not used.

## 3. Implementation steps

1. Edit `crates/gamelog-core/Cargo.toml`. Add optional `wasm-bindgen` `0.2.129`. Add direct `getrandom` `0.4.3`. Add feature `wasm` that enables `wasm-bindgen`, `uuid/js`, and `getrandom/wasm_js`. Set crate types `rlib` and `cdylib`. Keep chrono default features.

2. Add `crates/gamelog-core/src/wasm_api.rs`. Gate it with `#[cfg(feature = "wasm")]`. Export the functions in section 4. Each returns a JSON string. Shape: `{"ok":true,"value":...}` or `{"ok":false,"error":"..."}`. Do not throw across the boundary.

3. Add `civil_date(utc_rfc3339, offset_east_seconds) -> String` and `set_last_played(entry_json, date_yyyy_mm_dd, offset_east_seconds) -> String` in `crates/gamelog-core/src/civil_date.rs`. Same calendar date keeps the old timestamp. A new date uses midnight at that offset. A blank date clears `last_played`.

4. Export `parse_library_json`, `write_library_json`, `import_csv_json`, `export_csv_json`, `visible_ids`, `partition_new_json`, `validate_title`, `validate_system`, `validate_hours`, `validate_release_date`, and `validate_rating`. `visible_ids` takes entries JSON, filter JSON, query, sort key, and reversed flag. Filter JSON is `{"kind":"all"}` or `{"kind":"system","name":"PC"}`.

5. Add `crates/gamelog-core/tests/wasm.rs` under `#[cfg(target_arch = "wasm32")]` using `wasm-bindgen-test`. Dev-depend on `wasm-bindgen-test` `0.3` only if `cargo info` shows that crate. If the version differs, pin the crates.io version and record it.

6. Add job `wasm` to `.github/workflows/ci.yml`. Steps: checkout v7, install stable, add target `wasm32-unknown-unknown`, install wasm-pack 0.15.0, run `wasm-pack test --node crates/gamelog-core`. Do not change the `packages` job.

7. Gitignore `crates/gamelog-core/pkg/`.

## 4. Structure

`wasm_api.rs` is the only wasm-bindgen file. `civil_date.rs` stays free of wasm types so native tests can call it. `visible_ids` uses `SortKey::compare` and `Entry::matches_query`. It does not reimplement sort.

`validate_hours` accepts a finite number greater than or equal to 0. `validate_rating` accepts 0 through 5, or null for clear. `validate_title` and `validate_system` reject empty trim.

## 5. Tests to write

Wasm node tests, with fixed strings:

- Parse the Hades sample from `PLAN.md`. `visible_ids` with sort `title` returns that id.
- Query `zelda` does not return Hades. Query `HAD` does.
- Hours sort puts a 50-hour row before a 10-hour row. Reversed hours keep title A-Z on a tie. Use titles `A` and `B` at 10 hours and `C` at 50.
- Unrated sorts after rated in both directions.
- CSV text `title,system\nCeleste,Switch\n` returns one entry. Status is `WantToPlay`.
- CSV with no system column returns `ok: false`.
- `set_last_played` with offset 0 and `2025-06-01` yields `2025-06-01T00:00:00Z` when the old value was null.
- The same call with the same civil date keeps a non-midnight timestamp.
- Offset `-18000` and date `2025-06-01` yields `2025-06-01T05:00:00Z`.
- `validate_hours` rejects `-1` and `inf`.

Native `civil_date` tests cover the same offsets without wasm.

## 6. Acceptance criteria

- [ ] `wasm-pack test --node crates/gamelog-core` passes.
- [ ] `cargo test --locked` still passes with default features.
- [ ] The wasm package does not link ureq or ratatui.

| Criterion | Test |
|---|---|
| Sort and search | wasm `visible_ids` tests |
| CSV | wasm import test |
| Last played offset | `set_last_played` tests |
| Terminal suite | `cargo test --locked` |

## 7. Feature map entry

```markdown
## Wasm library API

The core crate exports JSON functions for sort, CSV, and version-1 library text.

Entry point: `wasm-pack test --node crates/gamelog-core`.

Use steps:
1. Build with feature `wasm`.
2. Call `visible_ids` with entries JSON and a query.

Check: `wasm-pack test --node crates/gamelog-core`
Expected: the command exits 0.

Code: `crates/gamelog-core/src/wasm_api.rs`, `crates/gamelog-core/src/civil_date.rs`
Tests: `crates/gamelog-core/tests/wasm.rs`
```

## 8. Manual check script

```sh
cargo test --locked -p gamelog-core
wasm-pack test --node crates/gamelog-core
```

Expected: both commands exit 0.

The commands contain no user text. Do not append an unquoted path.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 01-core-crate): core modules and `script/check-file-coverage.py` exist.
- Checked: `wasm-bindgen` 0.2.129 and `wasm-pack` 0.15.0.
- Checked: `getrandom` 0.4.3 feature name `wasm_js`. `uuid` 1.26.1 feature name `js`.
- Checked: chrono 0.4.45 default features include `wasmbind`. Exports still take an offset so node tests do not need `Local`.
- UNCHECKED: the current crates.io version of `wasm-bindgen-test`. Pin the version you observe. Do not guess a version in the acceptance text.
