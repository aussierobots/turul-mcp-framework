#!/usr/bin/env bash
# Reproduce docs.rs for every published crate that declares [package.metadata.docs.rs]:
# nightly rustdoc, --cfg docsrs, warnings denied, with exactly the features listed there.
set -euo pipefail
cd "$(dirname "$0")/.."
command -v rustup >/dev/null && rustup run nightly rustc --version >/dev/null 2>&1 \
  || { echo "  SKIP: no nightly toolchain"; exit 0; }
fail=0
checked=0
target="${CARGO_TARGET_DIR:-$(cargo metadata --format-version 1 --no-deps | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')}-docsrs"
while read -r name features; do
  echo "  docs.rs: $name [$features]"
  checked=$((checked + 1))
  RUSTDOCFLAGS="--cfg docsrs -D warnings -D missing_docs" CARGO_TARGET_DIR="$target" \
    cargo +nightly doc -q --no-deps -p "$name" --features "$features" || { echo "  FAIL: $name"; fail=1; }
done < <(cargo metadata --format-version 1 --no-deps | python3 -I -c '
import json, sys
for p in json.load(sys.stdin)["packages"]:
    d = ((p.get("metadata") or {}).get("docs") or {}).get("rs") or {}
    if d.get("features"):
        print(p["name"], ",".join(d["features"]))')
[ "$checked" -gt 0 ] || { echo "  FAIL: no crate declares [package.metadata.docs.rs] features"; exit 1; }
exit $fail
