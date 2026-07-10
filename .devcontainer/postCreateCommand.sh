#!/usr/bin/env bash

set -eu

# Install poetry and the Python package (with optional extras, e.g. pycairo).
python3 -m pip install --user poetry
python3 -m poetry install --all-extras

# Install JS dependencies for the website workspace.
cd website
npm ci
