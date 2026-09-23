# Roadmap

A running record of what's done and what's left, across both front ends.
gamelog has two interfaces — a terminal UI (ratatui) and an optional desktop
GUI (egui) — sharing the same `App` state machine and the same on-disk data
(`~/.gamelog/entries.json`, `covers/`, `settings.toml`). Anything under
"Core / shared" applies to both automatically, since it lives in `App`
itself rather than either rendering layer.

## Completed

### Core / shared (used by both front ends)

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

### GUI (egui/eframe, `--gui`)

- Full feature parity with the TUI: browsing/filtering, entry CRUD,
  sessions (with a live-counting button label), the first-run wizard, API
  key entry, cover fetch fallback choices, and manual import via a native
  file picker (`rfd`).
- All dialogs are true modals (`egui::Modal`) that block background
  interaction and are centered over the main window by default.
- Keyboard focus management for text-input dialogs: auto-focuses the input
  on open or when switching to a different kind of dialog, without
  trapping Tab navigation to the buttons.
- Cover art texture cache correctly invalidated after import, delete, and
  any completed fetch (single or bulk) — fixed after initial release; see
  commit `cafade8`.
- Search box (with Clear), sort dropdown, and Reverse toggle in the list
  panel, plus last played in the detail pane.
- Import CSV... / Export CSV... toolbar buttons using native open/save
  dialogs; the save dialog's own overwrite confirmation applies.
- Native file dialogs (CSV and cover art Browse...) run on a background
  thread, so the window keeps repainting while one is open instead of
  being flagged "Application Not Responding" by the compositor.

## Remaining

### Core / shared

- **Design choice to revisit**: *last played* is only set by stopping a play
  session; editing hours by hand doesn't touch it, and there's no way to
  set it directly.

### Terminal UI

- **Investigate**: the first keypress after launch is dropped when running
  under tmux (seen on builds before and after search/sort, so not a
  regression). Likely leftover input from the terminal graphics-protocol
  query at startup; not yet checked in a plain terminal.

### GUI

- **Bug (needs a decision)**: in normal mode, Esc calls `App::quit()`, but
  the GUI never checks `should_quit`, so the window stays open. Since
  `quit()` finalizes any running play session, Esc silently stops the
  session timer. Either Esc should close the window (like the TUI), or it
  should do nothing in normal mode.
- **Minor**: `draw_api_key_dialog`'s focus handling uses a simpler "focus if
  nothing else is focused" check rather than the discriminant-tracking fix
  applied to the general text-input dialog. In the rare case of two
  consecutive `EnterApiKey` dialogs for different sources with no
  intervening dialog, focus could theoretically land on the stale OK
  button instead of the new field. Not reachable through normal app flow
  today (mode transitions don't produce that sequence), so it's tracked
  here rather than fixed proactively.
- **Cosmetic**: after an action that ends a dialog mid-frame (e.g.
  `commit_import_cover` flipping `mode` back to `Normal`), the modal
  finishes rendering for that one frame before closing on the next.
  Harmless, one-frame flicker at most.
- **Housekeeping**: `GamelogApp::textures` isn't pruned of ids that no
  longer exist in the library outside of a delete (which is now handled).
  Not user-visible, since UUIDs aren't reused, but worth a periodic prune
  if the cache's unbounded growth ever becomes a concern for very long
  sessions.
- No `keybindings.toml` equivalent — this is an intentional design
  decision (see README's "GUI mode" section), not a gap: the GUI is
  buttons-and-dialogs by design, so there's nothing to remap.

### Possible future milestones (not yet started, not committed to)

- Packaging/distribution (e.g. prebuilt binaries or a Homebrew/AUR
  package) beyond `cargo install`.
