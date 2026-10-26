#!/bin/sh
# Runs `cargo package` (default) or `cargo publish` with the given arguments.
#
# The package includes the gitignored vendor/ and go/vendor/ trees, which cargo reports as
# uncommitted changes. So this checks that the tracked files are clean by itself and passes
# --allow-dirty, which would otherwise also let real uncommitted changes through.
set -eu
cd "$(dirname "$0")"
if [ -n "$(git status --porcelain)" ]; then
  git status --short >&2
  echo "commit or stash the changes above first" >&2
  exit 1
fi
./fetch.sh
[ $# -gt 0 ] || set -- package
cmd="$1"; shift
exec cargo "$cmd" --allow-dirty "$@"
