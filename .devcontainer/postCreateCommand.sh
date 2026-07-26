#!/usr/bin/env bash

set -eu

# Install poetry and the Python package (with optional extras, e.g. pycairo).
python3 -m pip install --user poetry
python3 -m poetry install --all-extras

# Install the WebAssembly build tools for Rust and the WASM version of PreFigure:
# the wasm32 compilation target and wasm-pack.
rustup target add wasm32-unknown-unknown
curl -sSfL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

# Install JS dependencies for the npm workspace. The repo root is the
# workspace root now (packages live under packages/), so run this from here.
npm ci
