// Post-build step for the RaTeX (rust-native) wasm builds.
//
// Both wasm builds come from the one crate, so wasm-pack regenerates the
// output `package.json` with the crate name ("prefig-wasm") -- identical to
// the default MathJax build. Rename it so the rust-native (RaTeX) build is a
// distinct publishable package with an accurate description. This runs on
// every native build, since wasm-pack overwrites the file each time.
//
// Usage: node scripts/label-native-pkg.mjs [out-dir]
//   out-dir defaults to "pkg-native" (the `--target nodejs` build); the web
//   build passes "pkg-web-native".

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const outDir = process.argv[2] ?? "pkg-native";
const pkgPath = join(here, "..", outDir, "package.json");

const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
pkg.name = "prefig-wasm-native";
pkg.description =
    "PreFigure compiled to WebAssembly with the rust-native RaTeX math backend " +
    "(no host MathJax required)";
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");

console.log(`labeled ${outDir} as "${pkg.name}"`);
