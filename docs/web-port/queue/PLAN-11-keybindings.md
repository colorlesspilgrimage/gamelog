# PLAN: 11-keybindings

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice stores custom keys and updates the help bar.

Non-goals:

- Do not read `~/.gamelog/keybindings.toml`.
- Do not bind Ctrl or Meta chords.
- Do not remove the fixed ArrowUp, ArrowDown, and Esc rules.

## 2. Repo context

ASSUMPTION (unmerged 04-sqlite-store): a settings table exists.
ASSUMPTION (unmerged 03-web-shell): list keys and the help bar exist.
ASSUMPTION (unmerged 05-edit-fields): add and field keys exist.
ASSUMPTION (unmerged 06-notes-rating-delete): notes, rating, and delete keys exist.
ASSUMPTION (unmerged 07-play-session): the `t` key starts a session.
ASSUMPTION (unmerged 08-import-export): `I` and `E` exist.
ASSUMPTION (unmerged 09-cover-images): `c` and `F` exist.
ASSUMPTION (unmerged 10-offline-pwa): the shell still loads with a service worker.

SETUP: same as slice 04.

FAST TEST: `cd web && npm test -- bindings && npm run test:e2e -- e2e/keybindings.spec.ts`

FULL TEST: cargo, wasm, web unit, and web e2e. CI-only: `packages`.

LINT: web check, lint, and format check.

COVERAGE: 85 percent on the diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node `>=22.17`.
- `box:` not used.

## 3. Implementation steps

1. Add column `keybindings_json` to `settings` through a new statement in `web/src/lib/db/bindings.ts`. Default JSON matches the terminal defaults in `src/keybindings.rs`: `j` down, `k` up, Tab next filter, Shift+Tab previous filter, Enter notes, `a` add, `d` delete, `s` status, `h` hours, `t` session, `r` rating, `c` cover, `T` title, `p` system, `R` release date, `L` last played, `F` fetch status, `/` search, `o` sort next, `O` sort reverse, `E` export, `I` import, `q` quit-noop.

2. Save bindings in the same `BEGIN IMMEDIATE` transaction as other settings writes.

3. Add `web/src/lib/ui/BindingDialog.svelte`. Open it with `?`. Each row names one action. Focus a row and press the new key. `?` is not a terminal default. It is the web way to open the screen because the terminal edits a file.

4. If two actions share a key, save nothing and show `Keybinding conflict: '<key>' is bound to both <a> and <b>.` Match the terminal warning text.

5. ArrowUp and ArrowDown always move the list in normal mode, even when `j` and `k` are rebound. Esc still clears a non-empty search before it does nothing. These match the fixed fallbacks in `src/main.rs`.

6. The help bar reads the stored labels. Search mode, notes mode, rating mode, and delete mode keep their fixed keys from earlier slices. Those modes are not configurable in the terminal either.

## 4. Structure

`web/src/lib/keys/defaults.ts` is the only default map. `bindings.ts` is the SQL. The dialog does not import SQLite. It calls the database client. Keep each file under 400 lines.

## 5. Tests to write

`web/src/lib/keys/conflict.test.ts`:

- `a` bound to both add and delete returns the conflict sentence.
- A map with unique keys returns null.

`web/e2e/keybindings.spec.ts` seeds three rows.

| Key | Result |
|---|---|
| `?` | Opens the binding dialog. |
| ArrowDown and ArrowUp inside the dialog | Move between actions. They do not change the library selection. |
| Focus `add`, press `n`, press Enter | `n` adds a row. `a` no longer adds. |
| Bind delete to `n` too | Conflict status. The stored map is unchanged. |
| Esc | Closes the dialog and does not quit. |
| ArrowDown in normal mode after `j` is rebound | Still moves down. |
| `j` after it was rebound to nothing | Does nothing, or does the new action if assigned. |
| `/` then `n` | Types `n` into the search when `n` is not a search control. It does not add a row. |
| Enter, `r`, `d`, `y`, `n` in their dialogs | Keep the fixed dialog meanings from slices 05 and 06. |
| `q` | Does nothing. |
| `x` when unbound | Does nothing. |

Reload. Expected: the custom add key still works.

## 6. Acceptance criteria

- [ ] Defaults match the terminal map.
- [ ] A custom key persists across reload.
- [ ] A duplicate binding is refused.
- [ ] Arrow keys still move the list.
- [ ] Dialog modes keep their fixed keys.

| Criterion | Test |
|---|---|
| Defaults and conflict | `conflict.test.ts` |
| Rebind add | `keybindings.spec.ts` |
| Fixed arrows | `keybindings.spec.ts` |
| Search does not add | `keybindings.spec.ts` |
| Reload | `keybindings.spec.ts` |

## 7. Feature map entry

```markdown
## Web key bindings

The `?` key edits action keys. The help bar shows the stored map. Arrow keys still move the list.

Entry point: `web/src/lib/ui/BindingDialog.svelte`.

Use steps:
1. Press `?`.
2. Choose Add. Press `n`. Confirm.
3. Press `n` on the list. A new-entry dialog opens.

Check: `cd web && npm run test:e2e -- e2e/keybindings.spec.ts`
Expected: the command exits 0.

Code: `web/src/lib/keys/defaults.ts`, `web/src/lib/db/bindings.ts`
Tests: `web/e2e/keybindings.spec.ts`
```

## 8. Manual check script

Use the preview or dev server. Press `?`. Set Add to `n`. Close with Esc. Press `n`. Expected: the new-entry dialog. Press Esc. Press `a`. Expected: no dialog. Reload. Press `n`. Expected: the dialog opens again.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 05-edit-fields) through ASSUMPTION (unmerged 10-offline-pwa): the actions in the default map exist.
- The terminal loads TOML with serde defaults. The web app stores JSON instead. The action names match the Rust field names.
- Do not accept a binding that includes Ctrl or Meta. Those chords belong to the browser.
- One settings write uses one `BEGIN IMMEDIATE` transaction. A second tab is already blocked by the SAH pool.
