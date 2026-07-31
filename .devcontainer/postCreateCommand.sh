#!/usr/bin/env bash

set -eu

# Install poetry and the Python package (with optional extras, e.g. pycairo).
python3 -m pip install --user poetry
python3 -m poetry install --all-extras

# Install the WebAssembly build tools for Rust and the WASM version of PreFigure:
# the wasm32 compilation target and wasm-pack.
rustup target add wasm32-unknown-unknown
curl -sSfL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

# Install a pinned Typst binary so the prefig-typst render tests run (the npm
# workspace test `packages/prefig-typst` -> tests/run.sh renders every fixture
# and asserts the pipeline invariants; without a `typst` on PATH it skips that).
# Pinned to the version the suite is verified against. Installed under
# ~/.local/bin to match the `pip install --user` convention above; both CI
# runCmds already put ~/.local/bin on PATH.
TYPST_VERSION=0.15.1
TYPST_ARCH=x86_64-unknown-linux-musl
mkdir -p "$HOME/.local/bin"
curl -sSfL "https://github.com/typst/typst/releases/download/v${TYPST_VERSION}/typst-${TYPST_ARCH}.tar.xz" \
  -o /tmp/typst.tar.xz
tar -xJf /tmp/typst.tar.xz -C /tmp
install -m 0755 "/tmp/typst-${TYPST_ARCH}/typst" "$HOME/.local/bin/typst"
rm -rf /tmp/typst.tar.xz "/tmp/typst-${TYPST_ARCH}"

# Install JS dependencies for the npm workspace. The repo root is the
# workspace root now (packages live under packages/), so run this from here.
npm ci
