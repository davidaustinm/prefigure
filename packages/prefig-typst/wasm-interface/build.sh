#!/usr/bin/env bash
# Build the PreFigure Typst plugin and drop the wasm next to the .typ sources so
# `plugin("prefig_typst_plugin.wasm")` in src/lib.typ finds it.
#
# Usage: ./build.sh              # default: no embedded math engine (~1.7 MiB)
#        ./build.sh --with-math  # embed RaTeX for baked SVG math (~5.4 MiB)
#
# Math is rendered by Typst (mitex) by default, so the shipped wasm needs no math
# engine. --with-math re-embeds RaTeX, which bakes math into the SVG so it needs
# nothing from the host; its font loader transitively pulls `system-fonts ->
# web-sys`, which emits
# `__wbindgen_*` host imports Typst rejects. Those paths are dead once fonts are
# embedded, so a post-build pass (tools/stub-imports) replaces them with trapping
# stubs, leaving only the two `typst_env` ABI imports. The default build pulls no
# such imports and needs no stubbing.
set -euo pipefail
cd "$(dirname "$0")"

WITH_MATH=0
if [[ "${1:-}" == "--with-math" ]]; then
  WITH_MATH=1
fi

OUT="target/wasm32-unknown-unknown/release/prefig_typst_plugin.wasm"
DEST="../src/prefig_typst_plugin.wasm"

if [[ "$WITH_MATH" == "1" ]]; then
  cargo build --release --target wasm32-unknown-unknown --features ratex-math
  echo "Stubbing non-ABI imports…"
  cargo build --release --manifest-path tools/stub-imports/Cargo.toml
  tools/stub-imports/target/release/stub-imports "$OUT" "$DEST"
else
  cargo build --release --target wasm32-unknown-unknown
  cp "$OUT" "$DEST"
fi

echo "Wrote $DEST ($(wc -c < "$DEST") bytes)"
