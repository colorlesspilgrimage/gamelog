# Roadmap

A running record of what's done and what's left. gamelog is a terminal UI
(ratatui) on top of an `App` state machine and plain on-disk data
(`~/.gamelog/entries.json`, `covers/`, `settings.toml`).

## Completed

### Core

- Entry model: title, system/platform, release date, status (*Want to
  Play* / *Playing* / *Played*), hours played, 0–5 star rating, cover art
  path, freeform notes, and a *last played* timestamp (set whenever a play
  session stops; older libraries load with it unset).
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
  system skipped as duplicates, and bad rows reported by line without
  aborting the import. Typed paths expand a leading `~`.

### Terminal UI (ratatui)

- Split-pane browsing: sortable list + detail pane, `Tab`/`Shift+Tab`
  to cycle the system filter.
- `/` opens a live-filtering search bar (`Enter` keeps it, `Esc` clears
  it; `Esc` in normal mode clears an active search before quitting).
  `o` cycles the sort key and `O` reverses it; the current sort is shown
  along the bottom of the list.
- Last played date shown in the Info pane.
- `I` / `E` prompt for a CSV path to import from / export to. A typed
  export path never overwrites an existing file.
- Cover art rendered inline (Kitty/iTerm2/Sixel graphics protocol, or a
  Unicode half-block fallback elsewhere).
- Configurable keybindings via `~/.gamelog/keybindings.toml`, generated
  with comments on first run; a live help bar reflects whatever's bound.
- Session timer shown prominently in the footer while running.

## Remaining

### Core

- **Design choice to revisit**: *last played* is only set by stopping a play
  session; editing hours by hand doesn't touch it, and there's no way to
  set it directly.

### Possible future milestones (not yet started, not committed to)

- Packaging/distribution (e.g. prebuilt binaries or a Homebrew/AUR
  package) beyond `cargo install`.
