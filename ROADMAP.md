# Roadmap

A running record of what's done and what's left. gamelog is a terminal UI
(ratatui) on top of an `App` state machine and plain on-disk data
(`~/.gamelog/entries.json`, `covers/`, `settings.toml`).

## Completed

### Core

- Entry model: title, system/platform, release date, status (*Want to
  Play* / *Playing* / *Played*), hours played, 0–5 star rating, cover art
  path, freeform notes, and a *last played* date (set when a play session
  stops, edited directly, or imported from CSV; older libraries load with
  it unset).
- Library storage as plain JSON in `~/.gamelog/entries.json`, with atomic
  (write-temp-then-rename) saves.
- Filtering by system, and an "All Entries" view.
- Title search: case-insensitive substring match, layered on top of the
  system filter.
- Sorting by title, hours, rating, release date, last played, or status,
  reversible, and remembered in `settings.toml`. Entries missing the
  sorted-on value always sink to the bottom; ties break by title. Title
  order is case-insensitive.
- The selected entry stays selected across edits, sort changes, and
  searches, since editing a sorted-on field can move it; a newly added
  entry clears any search that would hide it.
- Full CRUD: add, edit (every field), delete (with confirmation).
- Play sessions: start/stop a live timer on an entry; elapsed time is added
  to its hours automatically; a session survives being on a different
  screen and is safely finalized on quit.
- Automatic cover art fetching from SteamGridDB or RAWG.io (configurable
  per-source API keys in `settings.toml`), or manual import from a local
  image file.
- First-run setup wizard for choosing a default cover art source.
- Fallback flow when automatic fetch finds nothing: try the other source,
  import manually, or skip.
- `Fetch All Covers`: backfills every entry missing cover art through a
  bounded background worker pool, with retry-with-backoff on HTTP 429.
- Fully asynchronous cover fetching — never blocks the UI thread.
- CSV export of the whole library (title, system, status, hours, rating,
  release date, last played, notes; not cover art) and a forgiving CSV
  import: only `title`/`system` required, any column order or header case,
  common status spellings accepted, rows matching an existing title +
  system skipped as duplicates, and bad rows skipped without aborting the
  import (the status bar reports the count and the first bad row's line
  number). Typed CSV paths expand a leading `~`; cover art import paths
  don't.

### Terminal UI (ratatui)

- Split-pane browsing: sortable list + detail pane, `Tab`/`Shift+Tab`
  to cycle the system filter.
- `/` opens a live-filtering search bar (`Enter` keeps it, `Esc` clears
  it; `Esc` in normal mode clears an active search before quitting).
  `o` cycles the sort key and `O` reverses it; the current sort is shown
  along the bottom of the list.
- Last played date shown in the Info pane; `L` edits it directly (blank
  clears it; re-entering the shown date keeps the original timestamp).
- `I` / `E` prompt for a CSV path to import from / export to. A typed
  export path never overwrites an existing file.
- Cover art rendered inline (Kitty/iTerm2/Sixel graphics protocol, or a
  Unicode half-block fallback elsewhere).
- Configurable keybindings via `~/.gamelog/keybindings.toml`, generated
  with comments on first run; a live help bar reflects whatever's bound.
- Session timer shown prominently in the footer while running.

### Distribution

- Tagged releases (`v*`) build prebuilt binaries for x86_64 Linux and
  Apple Silicon/Intel macOS via GitHub Actions and attach them, with
  SHA-256 checksums, to a GitHub Release (`.github/workflows/release.yml`).
- x86_64 `.deb` and `.rpm` packages built by the same release workflow
  (metadata in `Cargo.toml`, built by `packaging/build-linux-packages.sh`).
- CI builds and tests every push and pull request to `master`, and builds
  the `.deb`/`.rpm`, installs them on Ubuntu 22.04 and Fedora, and checks the
  installed binary starts.
- An AUR `PKGBUILD` in `packaging/aur/` (not yet published to the AUR);
  release steps are in `packaging/RELEASING.md`.
