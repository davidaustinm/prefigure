#!/usr/bin/env bash
# Run the prefig-typst test suite: the native protocol tests (host Rust) and the
# Typst-native render/assert test (needs a `typst` binary + the built wasm).
#
# Typst binary resolution: $TYPST, else `typst` on PATH. Set TYPST to a checkout
# build if you don't have typst installed, e.g.
#   TYPST=/path/to/typst ./run.sh
set -euo pipefail
cd "$(dirname "$0")"
PKG_DIR="$(cd .. && pwd)"
REPO_ROOT="$(cd ../../.. && pwd)"

# 1. Native protocol tests (no wasm, no typst). The plugin's default feature set
#    embeds no math engine, so this needs neither RaTeX nor system fonts.
echo "== native protocol tests =="
( cd ../wasm-interface && cargo test --quiet )

# 2. Typst-native render + assertions.
TYPST_BIN="${TYPST:-$(command -v typst || true)}"
if [[ -z "$TYPST_BIN" ]]; then
  echo "== typst render tests: SKIPPED (no typst; set \$TYPST) =="
  exit 0
fi
if [[ ! -f "$PKG_DIR/src/prefig_typst_plugin.wasm" ]]; then
  echo "error: src/prefig_typst_plugin.wasm not built. Run wasm-interface/build.sh first." >&2
  exit 1
fi

echo "== typst render tests ($("$TYPST_BIN" --version)) =="
OUT="$(mktemp -d)/render.png"
"$TYPST_BIN" compile --root "$REPO_ROOT" render.typ "$OUT"
echo "OK — rendered to $OUT"
