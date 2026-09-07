# Building the monorepo

This covers building and running the JS/Rust/Python monorepo (the
playground, the language server, the Rust->Wasm core, and the Python
package) — as opposed to just `pip install`-ing `prefig` (see
`README.md` for that).

The canonical list of required tools is `.devcontainer/postCreateCommand.sh`;
this document explains what those steps produce and how to use the result.

## Prerequisites

| Tool | Why | Check |
|---|---|---|
| Node.js + npm | JS workspaces (playground, prefig-lsp, prefig-typst) | `node -v`, `npm -v` |
| Python 3.12 + Poetry | `packages/prefig` Python package | `poetry --version` |
| Rust (stable) + `wasm32-unknown-unknown` target | `packages/prefig-rust` compiled to Wasm for the browser | `rustc --version` |
| `wasm-pack` | Drives the Rust->Wasm build (`wasm-pack build --target web`) | `wasm-pack --version` |

If Rust/wasm-pack aren't installed yet:

```bash
curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
source "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown
curl -sSfL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
```

These install into `~/.cargo` and `~/.rustup` — no sudo needed. Open a new
shell (or re-`source "$HOME/.cargo/env"`) afterwards so `cargo`/`wasm-pack`
are on `PATH`.

Optional, only needed for the full devcontainer parity (native labels,
`prefig-typst` render tests): `libcairo2-dev`, `librsvg2-bin` (`rsvg-convert`),
and a pinned `typst` binary — see `.devcontainer/postCreateCommand.sh` for
exact versions/URLs. Not required to build or run the playground.

## Install dependencies

```bash
npm ci                       # JS workspaces: packages/playground, prefig-lsp, prefig-typst
poetry install --all-extras  # Python package: packages/prefig
```

## Build everything

```bash
npm run build
```

This runs, in order:

1. **`build:js`** -> `npm run build --workspace packages/playground`, which
   is a `wireit`-managed pipeline:
   - `../prefig-wasm:build:web` — compiles `packages/prefig-rust` to Wasm
     twice via `wasm-pack build --target web`: once as `pkg-web` (plain) and
     once as `pkg-web-native` (with the `ratex` feature). This step needs
     the Rust toolchain above and can take ~1 minute the first time.
   - `../..:build:py` — builds the Python wheel/sdist (`poetry build`,
     wrapped in `wireit` at the repo root) into `dist/`.
   - `tsc -b && vite build` — typechecks and bundles the React playground
     into `packages/playground/dist/`.
2. **`build:py`** — same `wireit` Python build as above (a no-op if
   `packages/prefig/**`, `pyproject.toml`, etc. haven't changed since the
   last build — `wireit` caches by content hash in `.wireit/`).

`wireit` skips steps whose declared inputs are unchanged, so re-running
`npm run build` after a small edit is fast — only the affected step(s)
rebuild.

## Run the playground

```bash
npm run dev --workspace packages/playground
```

This starts Vite's dev server (default `http://localhost:5173/`) with the
same `wireit` dependency on `prefig-wasm:build:web` and the Python wheel, so
the first `dev` run pays the same Rust/Wasm/Python build cost as `build`
above; subsequent runs reuse the cache and start in under a second.

The playground is a client-only app: it loads Pyodide (Python-in-the-browser)
on first page load to run PreFigure's Python compiler against the XML/YAML
source in the editor. That load takes roughly 15-20 seconds on a cold cache
(subsequent loads are faster via browser cache) — the "Loading Pyodide..."
message on the right pane is expected during this window, not a hang.

To stop the dev server, kill the process holding port 5173:

```bash
lsof -ti:5173 -sTCP:LISTEN | xargs -r kill
```

(`npm run dev` forks a child process; killing the `npm` PID directly does not
reliably stop the underlying Vite server. If you started it in the background
with `&`/`nohup`, note that killing only the port's listener can leave the
parent `npm`/`wireit` shell processes running as orphans — `pkill -f` matching
your exact `npm run dev` command line, or killing the specific PIDs from `ps`,
cleans those up too.)

### Auto-rebuild on Python changes

By default, `npm run dev` builds the Python wheel once at startup and does not
watch `packages/prefig/**` afterwards. To have edits to the Python source
automatically trigger a wheel rebuild and dev-server restart, set
`WIREIT_WATCH=true`:

```bash
WIREIT_WATCH=true npm run dev --workspace packages/playground
```

`wireit`'s watch mode monitors the full dependency graph declared in each
`wireit` script's `files`, not just the playground's own sources. Since `dev`
depends on `../..:build:py`, whose `files` include `packages/prefig/**`, an
edit there triggers `poetry build` -> new `dist/prefig-*.whl` ->
`vite-plugin-static-copy` copies the new wheel into `assets/pyodide` -> the
Vite dev service restarts.

Because Pyodide loads the wheel once per page load, you still need to
**manually refresh the browser tab** after each rebuild — the watch/restart
cycle updates the server, not the already-running Pyodide instance in an open
tab.

## Tests

```bash
npm run test      # JS (vitest, all workspaces) + Python (pytest)
npm run test:js
npm run test:py
```

## Known non-blocking issue

On a fresh playground load you may see a console error:

```
PreFigure language server failed to start: _ResponseError: Pending response rejected since connection got disposed
```

This is the CodeMirror LSP integration (editor autocomplete/diagnostics,
`packages/prefig-lsp`) failing to attach; it does not affect compiling or
rendering diagrams, which runs entirely through the Pyodide/Wasm path.
