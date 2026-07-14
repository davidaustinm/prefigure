#!/usr/bin/env bash

set -eu

# Install poetry and the Python package (with optional extras, e.g. pycairo).
python3 -m pip install --user poetry
python3 -m poetry install --all-extras

# Install the WebAssembly build tools for rust/prefig-wasm:
# the wasm32 compilation target and wasm-pack.
rustup target add wasm32-unknown-unknown
curl -sSfL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

# Install JS dependencies for the website workspace.
cd website
npm ci
