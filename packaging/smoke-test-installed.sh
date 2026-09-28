#!/usr/bin/env bash
# Checks that an installed gamelog (default /usr/bin/gamelog, or the path given
# as $1) starts and draws its main screen. Uses a throwaway HOME so no real
# library is touched. Requires `script` (util-linux) for the pseudo-terminal
# the TUI needs.
set -euo pipefail

bin=${1:-/usr/bin/gamelog}

home=$(mktemp -d)
trap 'rm -rf "$home"' EXIT
mkdir -p "$home/.gamelog"
# Skip the first-run wizard so the main screen is what gets drawn.
printf 'default_cover_source = "manual"\n' > "$home/.gamelog/settings.toml"

# gamelog runs until quit, so `timeout` ends it once the screen has had time
# to draw (startup can wait up to 2s for a terminal graphics reply). Without a
# real terminal on stdin the pseudo-terminal starts at 0x0, so size it first.
HOME="$home" TERM=xterm-256color timeout 5 \
  script -qec "stty rows 30 cols 120; exec '$bin'" /dev/null < /dev/null > "$home/screen.txt" || true

# ratatui skips over spaces with cursor moves, so match a single word: the
# "Sort:" label on the entry list's border only exists on the main screen.
if grep -q "Sort:" "$home/screen.txt"; then
  echo "gamelog started and drew the main screen"
else
  echo "gamelog did not draw the main screen; captured output:" >&2
  cat -v "$home/screen.txt" >&2
  exit 1
fi
