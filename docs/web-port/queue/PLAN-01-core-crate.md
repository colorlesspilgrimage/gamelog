# PLAN: 01-core-crate

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice moves pure rules into `crates/gamelog-core`. The terminal app keeps the same behavior.

Non-goals:

- Do not add wasm-bindgen.
- Do not add `web/`.
- Do not change key names, JSON field names, or version 1.
- Do not format unrelated files.
- Do not delete the terminal UI.

## 2. Repo context

Stack: Rust edition 2024, one package `gamelog` at the repo root. Pure rules are in `src/entry.rs` (354 lines) and `src/csv_io.rs` (293 lines). `src/app.rs` is 1175 lines. `src/ui.rs` is 522 lines. Do not add functions to those two files.

SETUP: Install Rust stable 1.85 or newer. Run `cargo fetch --locked`.

FAST TEST: `cargo test --locked`

FULL TEST: `cargo test --locked`. CI-only: `.github/workflows/ci.yml` job `packages`.

LINT: none.

COVERAGE: 85 percent lines on each changed production file. File list command: `git diff --name-only master...HEAD`. Tool: `cargo llvm-cov` from crate `cargo-llvm-cov` 0.9.1. Then run `python3 script/check-file-coverage.py`.

SHARED DEPENDENCY DIRECTORIES: `target/`.

Environment facts:

- `cloud-agent VM:` default Cargo 1.83.0 cannot parse edition 2024. rustc 1.99.0 can. `cargo +stable test --locked` passed with 19 tests.
- `box:` not used.

## 3. Implementation steps

1. Edit root `Cargo.toml`. Add `[workspace]` with `members = [".", "crates/gamelog-core"]` and `resolver = "3"`. Keep `[package] name = "gamelog"`. Keep the deb and rpm metadata.

2. Add `crates/gamelog-core/Cargo.toml`. Name the package `gamelog-core`. Set `edition = "2024"`. Library name `gamelog_core`. Crate type `rlib` only. Depend on `serde` `1.0.229` with `derive`, `serde_json` `1.0.151`, `chrono` `0.4.45` with `serde`, `uuid` `1.26.1` with `v4` and `serde`, and `csv` `1.4.0`. Dev-depend on `proptest` `1.12.0`.

3. Move `src/entry.rs` to `crates/gamelog-core/src/entry.rs`. Move `src/csv_io.rs` to `crates/gamelog-core/src/csv_io.rs`. Change their crate-local imports to module imports. Do not add tests to `entry.rs`. It is close to 400 lines.

4. Add `crates/gamelog-core/src/library_file.rs`. Define `parse_library(text: &str) -> Result<Vec<Entry>, String>` and `write_library(entries: &[Entry]) -> Result<String, String>`. Accept version 1 only. `write_library` uses pretty JSON. Missing `notes` becomes empty. Missing `last_played` becomes null.

5. Add `crates/gamelog-core/src/duplicates.rs`. Add `partition_new(existing: &[Entry], incoming: Vec<Entry>) -> (Vec<Entry>, usize)`. The key is lowercased title and lowercased system. Keep the first incoming row. Return the skip count.

6. Add `crates/gamelog-core/src/lib.rs`. Export `entry`, `csv_io`, `library_file`, and `duplicates`. Keep `lib.rs` under 80 lines.

7. Add `crates/gamelog-core/tests/sort_props.rs`. Property tests live there, not in `entry.rs`.

8. Edit `src/storage.rs`. Call `parse_library` and `write_library`. Keep directory creation, temp-file write, `sync_all`, and `rename` in this file.

9. Add `src/import_apply.rs`. Move the duplicate loop out of `App::import_csv` into `apply_import(existing, parsed) -> ApplyResult`. Call `partition_new`. Keep file open and status text in `src/app.rs`. The net line count of `src/app.rs` must fall.

10. Edit `use` paths in `src/app.rs`, `src/settings.rs`, `src/ui.rs`, and `src/main.rs`. Remove `mod entry` and `mod csv_io`. Do not add a function to `src/ui.rs` or `src/app.rs`.

11. Add `script/check-file-coverage.py`. It reads `--files` and `--report`. Format `llvm` reads cargo-llvm-cov JSON. A changed production `.rs` file under 85 percent lines fails. Skip `tests/` and skip non-code files. Do not skip a file because a human listed it.

12. Add `docs/FEATURES.md` with the two entries in section 7.

13. Run `cargo test --locked`. Run `cargo build --release --locked`. Confirm `target/release/gamelog` exists.

## 4. Structure

The core crate is pure. It has no `ratatui`, `crossterm`, `ureq`, `image`, or `directories` dependency. The terminal package depends on `gamelog-core = { path = "crates/gamelog-core" }`. Packaging stays valid because the binary package name and root path stay `gamelog`.

## 5. Tests to write

Keep every existing test. They must pass after the move.

Add `crates/gamelog-core/tests/library_file.rs`:

- The section 2 sample from `PLAN.md` parses to one Hades entry. Rating stars are 5. Cover path ends in `.jpg`.
- An entry object with no `last_played` and no `notes` parses. `last_played` is null. `notes` is empty.
- `version` 2 returns an error that contains `version`.
- Pretty output parses back to the same titles.

Add tests in `crates/gamelog-core/src/duplicates.rs`:

- `Hades` / `PC` skips `hades` / `pc`.
- Two new rows with the same key keep the first. The skip count is 1.
- Different systems both stay.

Add `crates/gamelog-core/tests/sort_props.rs`:

- For generated lists, a missing rating sorts after every present rating. Both directions do this.
- Equal hours sort by title A-Z when reversed is true and when it is false.
- Use proptest. Do not use a time seed.

`src/storage.rs` round-trip test stays. It still writes `entries.json` under a temp directory.

## 6. Acceptance criteria

- [ ] `cargo test --locked` passes.
- [ ] `target/release/gamelog` exists after a release build.
- [ ] A version-1 file from `write_library` loads through `Library::load` logic.
- [ ] Duplicate CSV rows still count as skipped.
- [ ] `src/app.rs` has fewer lines than 1175.

| Criterion | Test |
|---|---|
| Old unit tests | `cargo test --locked` |
| JSON version 1 | `library_file.rs` tests |
| Duplicates | `duplicates.rs` tests |
| Sort invariants | `sort_props.rs` |
| Binary path | manual check |

## 7. Feature map entry

```markdown
## Terminal library

A keyboard terminal UI logs games in `~/.gamelog/entries.json`.

Entry point: the `gamelog` binary from `cargo run`.

Use steps:
1. Run `gamelog`.
2. Press `a`. Type a title. Press Enter. Type a system. Press Enter.
3. Read the new title in the list.

Check: `cargo test --locked`
Expected: the summary contains `0 failed`.

Code: `src/main.rs`, `src/app.rs`, `src/ui.rs`, `crates/gamelog-core`
Tests: `cargo test --locked`

## Library core

Pure Rust sorts, searches, parses CSV, and parses version-1 library JSON.

Entry point: crate `gamelog_core`.

Use steps:
1. Call `library_file::parse_library` with a version-1 JSON string.
2. Read the `Entry` values it returns.

Check: `cargo test --locked -p gamelog-core`
Expected: the summary contains `0 failed`.

Code: `crates/gamelog-core/src/`
Tests: `crates/gamelog-core/tests/`
```

## 8. Manual check script

```sh
cargo test --locked
cargo build --release --locked
test -x target/release/gamelog
```

Expected: tests report `0 failed`. The `test -x` command exits 0.

Write no user text into this shell. Paths are fixed.

## 9. Risks and open assumptions

- Checked: rustc 1.99.0 builds this repo. `cargo +stable test --locked` result was 19 passed.
- Checked: `proptest` 1.12.0 needs rustc 1.88. Stable 1.99.0 is enough.
- Checked: `cargo-llvm-cov` 0.9.1 exists. This planning run did not execute llvm-cov.
- `resolver = "3"` matches edition 2024 on Cargo 1.99. If an older allowed Cargo rejects it, use `2` and record that in the slice report.
- Do not put new tests in `src/entry.rs` after the move if the file is at or over 400 lines.
