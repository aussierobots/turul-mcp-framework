#!/usr/bin/env bash
# Every dependency between published framework crates names the sibling's
# current version (docs/rules/crate-versioning.md). A bare "0.4" lets cargo
# pair a crate with an older sibling that lacks an API it calls.
#
# Checks normal and build dependencies of every publishable workspace crate
# that point at another workspace crate by path. The frozen 0.3 crates are
# exempt: they never move, so "0.3" is exact for them.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo metadata --format-version 1 --no-deps | jq -r '
  (.packages | map({key: .name, value: .version}) | from_entries) as $ver
  | .packages[]
  | select(.publish != [] and (.manifest_path | test("/crates/")))
  | . as $pkg
  | .dependencies[]
  | select(.path != null and .kind != "dev" and ($ver[.name] != null))
  | select(($ver[.name] | startswith("0.3.")) | not)
  | select(.req != ("^" + $ver[.name]))
  | "\($pkg.name) -> \(.name): requires \(.req), current is \($ver[.name])"
' > /tmp/check-internal-pins.$$ || true

if [ -s /tmp/check-internal-pins.$$ ]; then
  echo "Internal pins that do not name the sibling's current version:"
  cat /tmp/check-internal-pins.$$
  rm -f /tmp/check-internal-pins.$$
  exit 1
fi
rm -f /tmp/check-internal-pins.$$
echo "internal pins: OK"
