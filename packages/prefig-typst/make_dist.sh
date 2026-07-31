#!/usr/bin/env bash
# Build a clean, up-to-date distribution of the prefigure package in
# dist/prefigure/<version>/ — the folder that gets copied into the
# typst/packages repository (under packages/preview/) to publish
# @preview/prefigure. The published package is *just the plugin's data*: the
# compiled wasm, the .typ sources, the manifest, the licence, and a portable
# README. The Rust crate under wasm-interface/, the examples, and the tests are
# not shipped — only their product (src/prefig_typst_plugin.wasm) is.
#
# Usage:
#   ./make_dist.sh --tag=<TAG>        e.g. ./make_dist.sh --tag=v0.1.0
#   ./make_dist.sh --commit=<HASH>    e.g. ./make_dist.sh --commit=b5fa409
#   ./make_dist.sh --no-tag
#
# Steps:
#   1. rebuild the wasm plugin from wasm-interface/ (the artifact that ships)
#   2. run the test suite
#   3. compile every example against the in-tree src/, as validation
#   4. regenerate the README screenshots from the examples
#   5. assemble the package (typst.toml, LICENSE, README.md, src/ — no
#      wasm-interface/, examples/, or tests/); the README's repo-relative
#      links/images (which only resolve when browsing this repo on GitHub) are
#      rewritten to absolute permalinks against `repository` in typst.toml,
#      pinned at --tag/--no-tag, so the published README is portable to Typst
#      Universe. Because this package lives in a subdirectory of its repo, the
#      permalinks include that subdirectory prefix (derived from git).
#   6. compile the examples against the vendored package (import rewritten to
#      `@preview/prefigure:<version>`) to validate it works as advertised
set -euo pipefail
cd "$(dirname "$0")"

usage() {
    cat >&2 <<'EOF'
usage: ./make_dist.sh (--tag=<TAG> | --commit=<HASH> | --no-tag)

This script rewrites README.md's repo-relative links (example files,
screenshots) into absolute GitHub permalinks, since the published package
does not ship the examples/ directory itself. A permalink must be pinned to
something — a release tag or a commit — so you must say which:

  --tag=<TAG>       Pin the links to the given git tag, e.g. --tag=v0.1.0. Use
                    this for an actual release: create and push the tag first
                        git tag v0.1.0 && git push origin v0.1.0
                    then pass that same tag here.

  --commit=<HASH>   Pin the links to a specific commit, e.g. --commit=b5fa409.
                    Any revision git can resolve is accepted (short/full hash,
                    or a ref like origin/main); it is expanded to the full
                    commit hash for the permalinks. Use this to pin a dist/ at
                    a known commit without creating a release tag.

  --no-tag          Pin the links to the current commit (git rev-parse HEAD)
                    instead of a tag. Use this for a local/test build of dist/
                    when you don't want to create a release tag yet.

Exactly one of these is required — there is no default, because silently
falling back to one could point a real release at an untagged commit, or
force a test build to fail for lacking a tag.
EOF
}

TAG_MODE=""
RELEASE_TAG=""
RELEASE_COMMIT=""
for arg in "$@"; do
    case "$arg" in
        --tag=*)
            TAG_MODE="tag"
            RELEASE_TAG="${arg#--tag=}"
            ;;
        --commit=*)
            TAG_MODE="commit"
            RELEASE_COMMIT="${arg#--commit=}"
            ;;
        --no-tag)
            TAG_MODE="no-tag"
            ;;
        *)
            echo "error: unknown argument '$arg'" >&2
            echo >&2
            usage
            exit 1
            ;;
    esac
done
if [ -z "$TAG_MODE" ]; then
    echo "error: --tag=<TAG>, --commit=<HASH>, or --no-tag is required" >&2
    echo >&2
    usage
    exit 1
fi
if [ "$TAG_MODE" = "tag" ] && [ -z "$RELEASE_TAG" ]; then
    echo "error: --tag= was given an empty tag name" >&2
    echo >&2
    usage
    exit 1
fi
if [ "$TAG_MODE" = "commit" ] && [ -z "$RELEASE_COMMIT" ]; then
    echo "error: --commit= was given an empty commit hash" >&2
    echo >&2
    usage
    exit 1
fi

# Typst binary resolution matches tests/run.sh: $TYPST, else `typst` on PATH.
TYPST_BIN="${TYPST:-$(command -v typst || true)}"
if [ -z "$TYPST_BIN" ]; then
    echo "error: no typst binary found. Install typst or set \$TYPST to a build." >&2
    exit 1
fi
# --root for compiling examples: the repo root, so the examples' relative
# read()/plugin() paths resolve exactly as in the dev flow.
REPO_ROOT="$(cd ../.. && pwd)"

VERSION=$(grep -m1 '^version' typst.toml | sed 's/.*"\(.*\)"/\1/')
PKG="dist/prefigure/$VERSION"
echo "==> prefigure $VERSION (typst: $("$TYPST_BIN" --version))"

# Base URLs for rewriting the README's repo-relative links (see step 5).
# Pinned to a permalink (a tag or a commit, never a branch name). This package
# lives in a subdirectory of its repository, so the permalinks must include
# that prefix — e.g. .../blob/<ref>/packages/prefig-typst/examples/... —
# otherwise the links would resolve against the repo root and 404.
REPO_URL=$(grep -m1 '^repository' typst.toml | sed 's/.*"\(.*\)"/\1/')
PKG_SUBDIR="$(git rev-parse --show-prefix)"   # e.g. "packages/prefig-typst/"
PKG_SUBDIR="${PKG_SUBDIR%/}"                   # drop trailing slash
if [ "$TAG_MODE" = "tag" ]; then
    if ! git rev-parse --verify --quiet "refs/tags/$RELEASE_TAG" >/dev/null; then
        echo "error: git tag '$RELEASE_TAG' does not exist in this repository." >&2
        echo >&2
        echo "  --tag=$RELEASE_TAG was given, but that tag hasn't been created." >&2
        echo "  Create and push it first:" >&2
        echo >&2
        echo "    git tag $RELEASE_TAG" >&2
        echo "    git push origin $RELEASE_TAG" >&2
        echo >&2
        echo "  ...then re-run this script." >&2
        exit 1
    fi
    GITHUB_REF="$RELEASE_TAG"
    if [ "$(git rev-parse "$RELEASE_TAG")" != "$(git rev-parse HEAD)" ]; then
        echo "warning: tag '$RELEASE_TAG' does not point at the current commit (HEAD)." >&2
        echo "         The README will link to tag '$RELEASE_TAG', but the examples and" >&2
        echo "         screenshots used to build this dist/ come from the current" >&2
        echo "         working tree, which may not match what that tag contains." >&2
    fi
elif [ "$TAG_MODE" = "commit" ]; then
    # Accept any revision git can resolve, but require it to be a real commit,
    # then expand to the full 40-char hash for a stable permalink.
    if ! GITHUB_REF=$(git rev-parse --verify --quiet "$RELEASE_COMMIT^{commit}"); then
        echo "error: '$RELEASE_COMMIT' does not resolve to a commit in this repository." >&2
        echo >&2
        echo "  --commit=$RELEASE_COMMIT was given, but git can't resolve it to a commit." >&2
        echo "  Pass a short/full commit hash (or a ref like origin/main) that exists here." >&2
        exit 1
    fi
    if [ "$GITHUB_REF" != "$(git rev-parse HEAD)" ]; then
        echo "warning: commit '$RELEASE_COMMIT' is not the current commit (HEAD)." >&2
        echo "         The README will link to commit '$GITHUB_REF', but the examples and" >&2
        echo "         screenshots used to build this dist/ come from the current" >&2
        echo "         working tree, which may not match what that commit contains." >&2
    fi
else
    GITHUB_REF=$(git rev-parse HEAD)
fi
if [ -n "$(git status --porcelain)" ]; then
    echo "warning: working tree has uncommitted changes; README links will point at" >&2
    echo "         $GITHUB_REF, which may not match what's on GitHub yet." >&2
    echo "         Commit and push before running this for a release." >&2
fi
REF_BASE="$GITHUB_REF${PKG_SUBDIR:+/$PKG_SUBDIR}"
BLOB_BASE="$REPO_URL/blob/$REF_BASE"
TREE_BASE="$REPO_URL/tree/$REF_BASE"
RAW_BASE="https://raw.githubusercontent.com/${REPO_URL#https://github.com/}/$REF_BASE"

echo "==> Rebuilding the wasm plugin"
# The single artifact this package ships. build.sh writes src/prefig_typst_plugin.wasm.
( cd wasm-interface && ./build.sh )

echo "==> Running tests"
TYPST="$TYPST_BIN" ./tests/run.sh

echo "==> Compiling examples (against in-tree src/)"
for f in examples/*.typ; do
    "$TYPST_BIN" compile --root "$REPO_ROOT" -f pdf "$f" /dev/null
done

echo "==> Rendering README screenshots"
"$TYPST_BIN" compile --root "$REPO_ROOT" -f png --ppi 130 examples/showcase.typ     examples/images/showcase.png
"$TYPST_BIN" compile --root "$REPO_ROOT" -f png --ppi 130 examples/quickstart.typ   examples/images/quickstart.png
"$TYPST_BIN" compile --root "$REPO_ROOT" -f png --ppi 130 examples/label-modes.typ  examples/images/label-modes.png
"$TYPST_BIN" compile --root "$REPO_ROOT" -f png --ppi 130 examples/math-by-typst.typ examples/images/math-by-typst.png

echo "==> Assembling $PKG/"
rm -rf dist
mkdir -p "$PKG"
cp typst.toml LICENSE "$PKG/"
# The published README keeps user documentation only: strip the Development
# section (everything from "## Development" to the next "## " heading). Then
# rewrite repo-relative links so the README is portable outside GitHub:
# markdown links `](path)` and `<a href="path">` become blob links (tree links
# for directory paths ending in `/`), and `<img src="path">` becomes a
# raw.githubusercontent.com link. Absolute/anchor/mailto targets are untouched.
awk '/^## Development$/ { skip = 1; next } skip && /^## / { skip = 0 } !skip' \
    README.md \
    | BLOB_BASE="$BLOB_BASE" TREE_BASE="$TREE_BASE" RAW_BASE="$RAW_BASE" perl -pe '
        s{\]\(([^)]+)\)}{
            my $p = $1;
            $p =~ m{^(?:https?:|#|mailto:)} ? "](" . $p . ")"
            : $p =~ m{/$} ? "](" . $ENV{TREE_BASE} . "/" . $p . ")"
            : "](" . $ENV{BLOB_BASE} . "/" . $p . ")"
        }ge;
        s{(<img\b[^>]*\bsrc=")([^"]+)(")}{
            $2 =~ m{^https?:} ? "$1$2$3" : "$1" . $ENV{RAW_BASE} . "/$2" . "$3"
        }ge;
        s{(<a\b[^>]*\bhref=")([^"]+)(")}{
            $2 =~ m{^https?:} ? "$1$2$3" : "$1" . $ENV{BLOB_BASE} . "/$2" . "$3"
        }ge;
    ' >"$PKG/README.md"
# Ship the .typ sources and the compiled wasm; nothing else from src/.
cp -r src "$PKG/src"

echo "==> Validating the vendored package as @preview/prefigure:$VERSION"
# Typst resolves `@preview/...` from `$TYPST_PACKAGE_PATH/preview/...`, so
# expose dist/ under a `preview` symlink and compile the examples against it —
# exactly how users will consume the package. The examples aren't part of the
# published package; a scratch copy of each, with the `../src/lib.typ` import
# rewritten to `@preview/prefigure:<version>`, is compiled here only to validate.
# The scratch copies are written *into examples/* (not a temp dir) so each
# example's relative `read("figures/...")` still resolves; they're removed after.
pkgroot=$(mktemp -d)
scratch=()
cleanup() { rm -rf "$pkgroot"; for s in "${scratch[@]}"; do rm -f "$s"; done; }
trap cleanup EXIT
ln -s "$PWD/dist" "$pkgroot/preview"
for f in examples/*.typ; do
    v="examples/.validate-$(basename "$f")"
    scratch+=("$v")
    # Rewrite only the import *path*, preserving whatever is imported
    # (`prefigure`, `prefigure, tags`, `*`, …).
    sed "s|\"../src/lib.typ\"|\"@preview/prefigure:$VERSION\"|" "$f" >"$v"
    TYPST_PACKAGE_PATH="$pkgroot" "$TYPST_BIN" compile --root "$REPO_ROOT" -f pdf "$v" /dev/null
done

echo "==> Done: $PKG/ ($(du -sh dist | cut -f1))"
find dist -type f | sort
