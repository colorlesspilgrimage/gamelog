# PLAN: 09-cover-images

## 1. Goal and non-goals

Feature prompt (exact): "Port gamelog (a Rust ratatui terminal app for logging video games: title, system, status, hours, 5-star rating, cover art, notes, data in JSON in a hidden home directory) to a full web app with feature parity, built for performance. Proposed stack: SvelteKit; the existing Rust core logic compiled to WebAssembly where it carries real logic; local-first storage with SQLite on OPFS in the browser; a service worker for offline use and installability; AVIF/responsive/lazy cover art; web workers for import and search; a keyboard-driven UI that keeps the TUI's keymap feel; and import of an existing gamelog JSON file. Decide whether the terminal app stays in the repo alongside the web app, and say why. Break the work into ordered slices."

This slice imports a local cover image and displays it.

Non-goals:

- Do not call SteamGridDB or RAWG.
- Do not require AVIF encode.
- Do not type a filesystem path. The browser file picker replaces the terminal path prompt.

## 2. Repo context

ASSUMPTION (unmerged 06-notes-rating-delete): delete exists.
ASSUMPTION (unmerged 04-sqlite-store): the database worker exists.

SETUP: same as slice 04.

FAST TEST: `cd web && npm test -- cover && npm run test:e2e -- e2e/cover.spec.ts`

FULL TEST: cargo, wasm, web unit, web e2e. CI-only: `packages`.

LINT: web check, lint, and format check.

COVERAGE: 85 percent on the diff file list.

SHARED DEPENDENCY DIRECTORIES: `target/`, `web/node_modules/`.

Environment facts:

- `cloud-agent VM:` Node `>=22.17`. Chromium from Playwright can run the image test.
- `box:` not used.

## 3. Implementation steps

1. Add table `covers` in `web/src/lib/db/covers.ts`:

```sql
CREATE TABLE covers (
  entry_id TEXT PRIMARY KEY,
  mime TEXT NOT NULL,
  bytes BLOB NOT NULL,
  thumb BLOB,
  thumb_mime TEXT,
  detail BLOB,
  detail_mime TEXT
);
```

2. Messages `putCover`, `getCover`, and `deleteCover` run in `BEGIN IMMEDIATE`. `deleteEntry` also deletes the cover row in that same transaction.

3. Reject a file over 15 MiB. Accept png, jpeg, webp, gif, and avif by sniffed mime. Reject other types with `Unsupported image type.`

4. Add `web/src/lib/images/derivatives.ts`. Decode with `createImageBitmap`. Draw a 240 px thumb and a 600 px detail on `OffscreenCanvas` in `web/src/lib/workers/image-worker.ts`. Probe `image/avif`, then `image/webp`, then `image/jpeg`. Keep a derivative only when `blob.type` equals the requested type. Always keep the original bytes.

5. The detail `<img>` sets `srcset` to thumb and detail, `sizes` to the pane width, `width`, `height`, `alt` to `<title> cover`, `loading="lazy"`, and `decoding="async"`. No cover shows `No cover art (press 'c' to import)`.

6. Key `c` opens the file picker. Key `F` sets status `Automatic cover fetch is not available in the web app.` It must not call `fetch`.

## 4. Structure

Image math stays in the worker. The database worker stores blobs. The component only builds object URLs and revokes them on destroy. Do not put image bytes in the service-worker cache. That cache does not exist yet.

## 5. Tests to write

`web/src/lib/images/derivatives.test.ts` with a fake encoder:

- When the probe returns `image/avif`, the stored detail mime is `image/avif`.
- When the probe returns `image/png` for an avif request, avif is not stored.
- A 15 MiB plus one byte file is rejected.

`web/e2e/cover.spec.ts` attaches a small real PNG.

| Key | Result |
|---|---|
| `c` and a PNG | The selected row shows an image. Reload still shows it. |
| `c` and a `.txt` file | Status says the type is unsupported. The old image stays. |
| `F` | Status says automatic fetch is not available. Network log has no `steamgriddb.com` and no `rawg.io`. |
| `d` then `y` | The image goes away with the row. |
| `j` | Moves the list. The image is not a path string. |
| Arrow keys | Move the list in normal mode. |

Assert the image element has `loading="lazy"` and a `srcset` with two candidates.

## 6. Acceptance criteria

- [ ] A picked PNG survives reload.
- [ ] AVIF is stored only when the probe agrees.
- [ ] The image is lazy and has thumb and detail candidates.
- [ ] `F` does not touch the network.
- [ ] Delete removes the cover row.

| Criterion | Test |
|---|---|
| PNG round trip | `cover.spec.ts` |
| AVIF probe | `derivatives.test.ts` |
| Lazy srcset | `cover.spec.ts` |
| F key | `cover.spec.ts` |
| Delete | `cover.spec.ts` |

## 7. Feature map entry

```markdown
## Web cover image

The `c` key stores a local image and the detail pane shows a lazy responsive image.

Entry point: `web/src/lib/images/derivatives.ts`.

Use steps:
1. Select a game. Press `c`. Choose a PNG.
2. Read the image in the detail pane.
3. Reload the page.

Check: `cd web && npm run test:e2e -- e2e/cover.spec.ts`
Expected: the command exits 0.

Code: `web/src/lib/db/covers.ts`, `web/src/lib/workers/image-worker.ts`
Tests: `web/e2e/cover.spec.ts`
```

## 8. Manual check script

Use the dev server and a local PNG of Hades cover art that you already have. Press `c` and choose it. Expected: the image is visible and the request log has no RAWG host and no SteamGridDB host. Press `F`. Expected status: `Automatic cover fetch is not available in the web app.`

## 9. Risks and open assumptions

- ASSUMPTION (unmerged 06-notes-rating-delete): `deleteEntry` exists and can gain the cover delete.
- Checked: MDN `toBlob` falls back to png when the type is unsupported. The test checks `blob.type`.
- UNCHECKED: which browsers encode AVIF. Acceptance does not require AVIF bytes.
- Do not fetch `cover_art` strings. CWE-918.
- Blob URLs are same-origin. CSP `img-src` must allow `blob:`.
