# Releasing gamelog

Releases are built by `.github/workflows/release.yml`, which runs whenever a
tag matching `v*` is pushed. It builds `--release --locked` binaries for
`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and `x86_64-apple-darwin`,
packages each as `gamelog-<tag>-<target>.tar.gz` (binary, `README.md`,
`LICENSE`) with a matching `.sha256` file, and attaches them to a new GitHub
Release for the tag.

## Steps

1. **Bump the version** in `Cargo.toml` (e.g. `0.2.0`), run `cargo build` so
   `Cargo.lock` picks it up, and commit both files to `master`.
2. **Tag and push**:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

   The tag must be `v` followed by the exact `Cargo.toml` version — the AUR
   package derives its source URL from it.
3. **Check the release**: wait for the *Release* workflow to finish and
   confirm the GitHub Release lists three `.tar.gz` archives and three
   `.sha256` files.
4. **Update the AUR package** in `packaging/aur/`:

   ```sh
   cd packaging/aur
   # set pkgver=0.2.0 and reset pkgrel=1 in PKGBUILD, then:
   updpkgsums                         # replaces sha256sums with the real tag tarball hash
   makepkg -f                         # build + run tests locally from the tag tarball
   namcap PKGBUILD ./*.pkg.tar.zst    # optional lint, if namcap is installed
   makepkg --printsrcinfo > .SRCINFO
   ```

   Commit `PKGBUILD` and `.SRCINFO` to this repository.
5. **Publish to the AUR** (once the `gamelog` AUR package exists): copy
   `PKGBUILD` and `.SRCINFO` into a clone of
   `ssh://aur@aur.archlinux.org/gamelog.git`, commit, and push.

For a packaging-only fix with no new upstream version, bump `pkgrel` instead of
`pkgver`, then regenerate `.SRCINFO`.
