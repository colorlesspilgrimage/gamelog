#!/usr/bin/env bash
# Builds .deb and .rpm packages (plus .sha256 files) from an already-built
# `cargo build --release --locked --target <triple>` binary. Package metadata
# lives in Cargo.toml under [package.metadata.deb] / [package.metadata.generate-rpm].
#
# Requires cargo-deb and cargo-generate-rpm on PATH.
#
# usage: packaging/build-linux-packages.sh <target-triple> <out-dir>
set -euo pipefail

target=$1
out=$2

mkdir -p "$out"
cargo deb --no-build --target "$target" --output "$out"
cargo generate-rpm --target "$target" --output "$out"

cd "$out"
for pkg in *.deb *.rpm; do
  sha256sum "$pkg" > "$pkg.sha256"
done
ls -l
