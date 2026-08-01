#!/usr/bin/env node
// build-docs.mjs — render the README's own Typst examples to the images it
// references, so the README is self-contained and can be edited independently:
// there is NO mirrored `examples/*.typ` for a README figure. The ```typ block
// in the README *is* the source of the figure that follows it.
//
// Two ways a README figure is declared:
//
//   1. Implicit (zero markup): a fenced ```typ code block immediately followed
//      by an <img> — nothing between them but whitespace and an optional <p …>
//      wrapper — is the source for that image. The image's `src=` is the
//      destination (resolved relative to the README). Blocks not followed by an
//      image (import snippets, the API signature, the xml-to-string example) are
//      documentation only and are never rendered.
//
//   2. Explicit marker: for a figure whose code should NOT appear next to the
//      image (e.g. a hero image at the top of the README), a fenced ```typ block
//      wrapped in an HTML comment tagged with a destination path:
//
//        <!-- build-docs-render: examples/images/showcase.png
//        ```typ
//        …source…
//        ```
//        -->
//
//      The comment is invisible in the rendered README, so the source lives in
//      the README (still editable there) without being shown, and the <img> that
//      displays it can sit anywhere. `read()` paths in such a block must be
//      root-relative (e.g. "/packages/prefig-typst/examples/figures/…"), since
//      the block is compiled from a temp dir, not from examples/.
//
// For each such block this script:
//   1. rewrites the `@preview/prefigure…` import to the active in-tree library
//      (src/lib.typ), so the doc renders against the code in this checkout;
//   2. wraps it in a small render preamble (auto-sized white page + a default
//      house font the block may override) — README snippets omit page setup on
//      purpose, so without this they would rasterize as a full A4 page;
//   3. writes it to a temp folder and compiles it to the image `src=` path.
//
// Typst binary: $TYPST, else `typst` on PATH (same resolution as tests/run.sh).
// Usage:  node scripts/build-docs.mjs         (via `npm run build-docs`)

import { readFile, writeFile, mkdir, rm } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { dirname, resolve, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const PKG_DIR = resolve(fileURLToPath(import.meta.url), "../.."); // packages/prefig-typst
const REPO_ROOT = resolve(PKG_DIR, "../..");
const README = join(PKG_DIR, "README.md");
const TMP = join(REPO_ROOT, "tmp", "build-docs");
const PPI = 130; // matches the resolution make_dist.sh used for these screenshots

// The active library, addressed root-relative so it resolves regardless of
// where the temp file lives (typst treats a leading "/" as relative to --root).
const LIB_IMPORT = "/" + relative(REPO_ROOT, join(PKG_DIR, "src/lib.typ"));

// Injected ahead of every extracted block. Auto page = crop to content; `fill:
// white` matches the committed screenshots. The margin is deliberately generous
// (not the ~10pt an example file uses): PreFigure labels are overlaid as native
// Typst text that routinely overhangs the diagram's own box — a label anchored
// near the plot edge extends past it — and such overflow is clipped at the page
// edge, not captured by the auto crop. 1.5cm of breathing room keeps ordinary
// overhang from clipping while staying reasonably tight (there is no image
// trimmer to fall back on here). No font is set, so figures render in Typst's
// default font; a block can still set its own `#set text(font: …)`.
const PREAMBLE =
  '#set page(width: auto, height: auto, margin: 1.5cm, fill: white)\n';

const typst = process.env.TYPST || "typst";

// Collect every figure to render, from both the explicit markers and the
// implicit block-before-<img> convention.
function findFigures(md) {
  const figures = [];
  const consumed = []; // char spans owned by an explicit marker

  // Explicit markers first, so their blocks are excluded from implicit pairing.
  const markerRe =
    /<!--\s*build-docs-render:\s*(\S+)\s*\r?\n```typ\r?\n([\s\S]*?)\r?\n```\s*-->/g;
  for (const m of md.matchAll(markerRe)) {
    figures.push({ src: m[1], code: m[2] });
    consumed.push([m.index, m.index + m[0].length]);
  }

  // Implicit: a ```typ block immediately followed by an <img>. Skip blocks that
  // live inside an explicit marker comment (they are already handled above).
  const blocks = [...md.matchAll(/```typ\r?\n([\s\S]*?)\r?\n```/g)]
    .map((m) => ({ code: m[1], start: m.index, end: m.index + m[0].length }))
    .filter((b) => !consumed.some(([s, e]) => b.start >= s && b.end <= e));
  const imgs = [...md.matchAll(/<img\b[^>]*?\bsrc="([^"]+)"[^>]*>/g)].map((m) => ({
    src: m[1],
    start: m.index,
  }));

  for (const img of imgs) {
    const prev = blocks.filter((b) => b.end <= img.start).at(-1);
    if (!prev) continue;
    // Only whitespace / a <p …> wrapper may sit between the fence and the image.
    const gap = md.slice(prev.end, img.start);
    if (!/^\s*(?:<p\b[^>]*>\s*)?$/.test(gap)) continue;
    figures.push({ code: prev.code, src: img.src });
  }
  return figures;
}

// Rewrite the package import to the in-tree library, keeping the imported names.
function useLocalLib(code) {
  return code.replace(/"@preview\/prefigure[^"]*"/g, `"${LIB_IMPORT}"`);
}

async function main() {
  const md = await readFile(README, "utf8");
  const figures = findFigures(md);

  if (figures.length === 0) {
    console.warn("build-docs: no README ```typ block is followed by an <img>; nothing to render.");
    return;
  }

  await rm(TMP, { recursive: true, force: true });
  await mkdir(TMP, { recursive: true });

  console.log(`build-docs: ${figures.length} README figure(s) → images (typst: ${typst})`);
  for (const [i, fig] of figures.entries()) {
    const srcFile = join(TMP, `fig-${i}.typ`);
    await writeFile(srcFile, PREAMBLE + "\n" + useLocalLib(fig.code) + "\n");

    const dest = resolve(PKG_DIR, fig.src); // <img src> is relative to the README
    await mkdir(dirname(dest), { recursive: true });

    const r = spawnSync(
      typst,
      ["compile", "--root", REPO_ROOT, "-f", "png", "--ppi", String(PPI), srcFile, dest],
      { stdio: "inherit" },
    );
    if (r.error && r.error.code === "ENOENT") {
      console.error(
        `build-docs: '${typst}' not found. Install Typst or set $TYPST to its path.`,
      );
      process.exit(1);
    }
    if (r.status !== 0) {
      console.error(`build-docs: failed to render figure ${i} → ${fig.src}`);
      process.exit(r.status ?? 1);
    }
    console.log(`  ✓ ${fig.src}`);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
