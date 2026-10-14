# PLAN: 10-offline-pwa

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice makes the built app installable and usable offline.

Non-goals:

- Do not cache third-party hosts.
- Do not put SQLite bytes into Cache Storage.
- Do not add screenshot baselines.

## 2. Repo context

ASSUMPTION (unmerged 03-web-shell): the static app builds.
ASSUMPTION (unmerged 04-sqlite-store): rows live in OPFS.
ASSUMPTION (unmerged 05-edit-fields): the UI can add a row, so the offline test needs no dev import.

SETUP: same as slice 04. Add a preview script that serves `web/build` and maps unknown paths to `web/build/index.html`.

FAST TEST: `cd web && npm run build && npm run test:e2e -- e2e/offline.spec.ts`

FULL TEST: cargo, wasm, web unit, and web e2e. CI-only: `packages`.

LINT: web check, lint, and format check.

COVERAGE: 85 percent on the diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node `>=22.17`. Playwright Chromium can toggle offline.
- `box:` not used.

## 3. Implementation steps

1. Add `web/src/service-worker/index.ts`. Import `build`, `files`, and `version` from `$service-worker`. On install, cache those paths under a cache name that includes `version`. On activate, delete caches whose names differ.

2. On fetch, serve a cached shell asset when the path is in that list. For any other request, use the network. Do not add a runtime cache for POST, for wasm from another origin, or for any host other than this origin.

3. Add `web/src/service-worker/tsconfig.json` that extends `$app/tsconfig/service-worker`. Exclude the service worker from the app `tsconfig.json`. Kit documents this split.

4. Add `web/static/manifest.webmanifest`. Set `name` to `gamelog`, `display` to `standalone`, and `start_url` to `/`. Add PNG icons at 192 and 512. Generate them with a small script in `web/scripts/write-icons.mjs`. Commit the PNG files. The script uses no network.

5. Link the manifest from `web/src/app.html`. Add `theme-color`.

6. Set a Content-Security-Policy in `app.html`. Start with `default-src 'self'; connect-src 'self'; img-src 'self' blob: data:; worker-src 'self' blob:; manifest-src 'self'`. Add `wasm-unsafe-eval` to `script-src` only if the browser blocks the wasm module. Record the final policy in the feature map. Do not add `https://api.rawg.io` or `https://www.steamgriddb.com`.

7. Add `web/scripts/preview-static.mjs`. It serves `web/build` on `127.0.0.1:4173`. A missing file returns `index.html`. Document this because `vite preview` fallback behavior is unchecked.

8. The service worker is absent in `vite dev`. Run the offline test against the preview server.

## 4. Structure

The worker file only precaches the shell. Library bytes stay in OPFS through the database worker. Icons stay in `web/static/` so Kit lists them in `files`.

## 5. Tests to write

`web/e2e/offline.spec.ts`:

1. Build, then open the preview URL.
2. Add a row with `a`, title `Hades`, system `PC`.
3. Wait until `navigator.serviceWorker.ready` resolves.
4. Call `context.setOffline(true)`.
5. Reload. Expected: the Hades row is visible.
6. Press `j` and `k`. Expected: they move when more than one row exists. With one row they do not throw.
7. Press `F`. Expected: the no-fetch status. The page stays usable.
8. Collect failed requests. Expected: none to `rawg.io` or `steamgriddb.com`.

Also assert `link[rel=manifest]` and icon files `192` and `512` in the build output.

Keys that must not act while offline: `q` must not close the page. `F` must not throw. Edit keys from earlier slices still follow their own plans when the database worker is open.

## 6. Acceptance criteria

- [ ] After one online load, an offline reload shows the library.
- [ ] The manifest and both icons are in the build.
- [ ] The service worker does not cache another origin.
- [ ] `q` does not close the page.

| Criterion | Test |
|---|---|
| Offline reload | `offline.spec.ts` |
| Manifest and icons | `offline.spec.ts` |
| No third-party cache | `offline.spec.ts` |
| Preview server | manual check |

## 7. Feature map entry

```markdown
## Offline web app

A service worker precaches the shell. The library stays in OPFS and opens offline.

Entry point: `web/src/service-worker/index.ts`.

Use steps:
1. Run `cd web && npm run build && node scripts/preview-static.mjs`.
2. Open `http://127.0.0.1:4173` once.
3. Stop the network and reload.

Check: `cd web && npm run test:e2e -- e2e/offline.spec.ts`
Expected: the command exits 0.

Code: `web/src/service-worker/index.ts`, `web/static/manifest.webmanifest`
Tests: `web/e2e/offline.spec.ts`
```

## 8. Manual check script

```sh
cd web && npm run build && node scripts/preview-static.mjs
```

Open `http://127.0.0.1:4173`. Add Hades on PC. Reload once while online. In the browser, set offline. Reload. Expected: Hades is on the list. The application panel offers install when the browser supports it. If it does not, record that. Do not fail the slice only because the browser hid the install control.

Stop the preview process with Ctrl+C.

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 05-edit-fields): `a` creates a row.
- Checked: Kit registers `src/service-worker/index.ts`. Source: https://svelte.dev/docs/kit/service-workers on 2026-10-14.
- Checked: the worker is not bundled during development. Test the production build.
- UNCHECKED: the exact CSP token set this wasm build needs. Do not put an unchecked CSP string in an acceptance criterion.
- UNCHECKED: `vite preview` SPA fallback. Use `web/scripts/preview-static.mjs`.
- Visual snapshots stay out of this slice. CI image `mcr.microsoft.com/playwright:v1.64.1-noble` exists if a later job adds them. Do not run `--update-snapshots` here.
