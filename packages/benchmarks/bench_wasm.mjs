// PreFigure: WASM build benchmark (Node).
//
// Times the WebAssembly build of PreFigure -- the same Rust core that the
// native CLI uses, compiled to wasm32 and driven from JavaScript exactly as the
// browser playground drives it, via `build_from_string`.
//
// To make the numbers comparable to the native Rust and Python CLIs (which
// render math labels with MathJax), this provides a host API that renders real
// MathJax *in-process* using the `mathjax-full` bundle that ships with prefig
// (prefig/core/mj_sre). That mirrors how the browser keeps MathJax warm, rather
// than spawning a fresh `node` MathJax subprocess per build the way the CLIs do
// -- so treat WASM as "the engine with math kept warm". Text metrics are
// approximated (no canvas/cairo in plain Node); that affects label *placement*,
// not the amount of engine work being timed. Output correctness is checked
// separately, not here.
//
// Usage:
//   node benchmarks/bench_wasm.mjs [--runs N] [--warmup N]
//                                  [--examples a.xml b.xml] [--json out.json]

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { dirname, join, basename } from "node:path";
import { performance } from "node:perf_hooks";

const HERE = dirname(fileURLToPath(import.meta.url));
const PACKAGES = dirname(HERE); // packages/
const EXAMPLES_DIR = join(PACKAGES, "tests", "examples", "hand_crafted");
const WASM_PKG = join(PACKAGES, "prefig-wasm", "pkg", "prefig_wasm.js");
const MJ_DIR = join(PACKAGES, "prefig", "core", "mj_sre");

const EXAMPLE_FILES = [
    "tangent.xml",
    "derivatives.xml",
    "de-system.xml",
    "diffeqs.xml",
    "implicit.xml",
    "projection.xml",
    "riemann.xml",
    "roots_of_unity.xml",
];

const DEFAULT_RUNS = 5;
const DEFAULT_WARMUP = 1;

// --------------------------------------------------------------------------
// CLI args
// --------------------------------------------------------------------------

function parseArgs(argv) {
    const opts = { runs: DEFAULT_RUNS, warmup: DEFAULT_WARMUP, examples: null, json: null };
    for (let i = 0; i < argv.length; i++) {
        const a = argv[i];
        if (a === "--runs") opts.runs = parseInt(argv[++i], 10);
        else if (a === "--warmup") opts.warmup = parseInt(argv[++i], 10);
        else if (a === "--json") opts.json = argv[++i];
        else if (a === "--examples") {
            opts.examples = [];
            while (i + 1 < argv.length && !argv[i + 1].startsWith("--")) {
                opts.examples.push(argv[++i]);
            }
        } else {
            console.error(`Unknown argument: ${a}`);
            process.exit(2);
        }
    }
    return opts;
}

// --------------------------------------------------------------------------
// Host API: real MathJax in-process, approximate text metrics.
// --------------------------------------------------------------------------

function makeHostApi() {
    // Load mathjax-full from prefig's bundled MathJax so we render with the
    // same library the native CLI uses (mj-sre-page.js is MathJax v3 too).
    const mjRequire = createRequire(join(MJ_DIR, "package.json"));
    const { mathjax } = mjRequire("mathjax-full/js/mathjax.js");
    const { TeX } = mjRequire("mathjax-full/js/input/tex.js");
    const { SVG } = mjRequire("mathjax-full/js/output/svg.js");
    const { liteAdaptor } = mjRequire("mathjax-full/js/adaptors/liteAdaptor.js");
    const { RegisterHTMLHandler } = mjRequire("mathjax-full/js/handlers/html.js");
    const { AllPackages } = mjRequire("mathjax-full/js/input/tex/AllPackages.js");

    const adaptor = liteAdaptor();
    RegisterHTMLHandler(adaptor);
    const svgDoc = mathjax.document("", {
        InputJax: new TeX({ packages: AllPackages }),
        OutputJax: new SVG({ fontCache: "none" }),
    });

    return {
        // Approximate glyph metrics: enough for the engine to do all its label
        // layout work; exact positioning would need canvas/cairo. Returns
        // [width, ascent, descent] in px, like the browser's measure_text.
        measure_text(text, fontString) {
            const m = /(\d+(?:\.\d+)?)px/.exec(fontString);
            const size = m ? parseFloat(m[1]) : 12;
            return [text.length * size * 0.5, size * 0.75, size * 0.25];
        },
        translate_text(text, _typeform) {
            return text;
        },
        processMath(tex) {
            const node = svgDoc.convert(tex, { display: false });
            return adaptor.outerHTML(node);
        },
        processBraille(_tex) {
            return "⠿";
        },
    };
}

// --------------------------------------------------------------------------
// Timing
// --------------------------------------------------------------------------

function stats(times) {
    if (times.length === 0) return { mean: null, std: 0 };
    const mean = times.reduce((a, b) => a + b, 0) / times.length;
    const variance =
        times.length > 1
            ? times.reduce((a, b) => a + (b - mean) ** 2, 0) / (times.length - 1)
            : 0;
    return { mean, std: Math.sqrt(variance) };
}

async function main() {
    const opts = parseArgs(process.argv.slice(2));

    let wasm;
    try {
        wasm = await import(WASM_PKG);
    } catch (e) {
        console.error(`[!!] Could not load WASM package at ${WASM_PKG}`);
        console.error("     Build it with: cd packages/prefig-wasm && npm run build");
        console.error(`     (${e.message})`);
        process.exit(1);
    }

    const { build_from_string, set_host_api, version } = wasm;

    let host;
    try {
        host = makeHostApi();
    } catch (e) {
        console.error("[!!] Could not initialize the MathJax host API.");
        console.error(`     Is prefig/core/mj_sre installed? Run \`prefig init\`. (${e.message})`);
        process.exit(1);
    }
    set_host_api(host);

    const names = opts.examples || EXAMPLE_FILES;
    const examples = names
        .map((n) => join(EXAMPLES_DIR, n))
        .filter((p) => {
            try {
                readFileSync(p);
                return true;
            } catch {
                console.error(`  [--] skipping missing example: ${basename(p)}`);
                return false;
            }
        });

    console.log("=".repeat(64));
    console.log("  PreFigure: WASM build benchmark (Node)");
    console.log("=".repeat(64));
    console.log(`  WASM prefig-core version: ${version()}`);
    console.log(`  Runs per example: ${opts.runs} (+${opts.warmup} warmup)`);
    console.log(`  Examples: ${examples.length}`);
    console.log();

    const results = [];
    for (let i = 0; i < examples.length; i++) {
        const path = examples[i];
        const name = basename(path).replace(/\.xml$/, "");
        const source = readFileSync(path, "utf8");
        process.stdout.write(`  [${i + 1}/${examples.length}] ${name} ... `);

        let ok = true;
        const times = [];
        for (let r = 0; r < opts.warmup + opts.runs; r++) {
            const t0 = performance.now();
            let svg;
            try {
                ({ svg } = build_from_string("svg", source));
            } catch (e) {
                ok = false;
                console.log(`FAILED: ${e.message}`);
                break;
            }
            const dt = performance.now() - t0;
            if (!svg || !svg.startsWith("<svg")) {
                ok = false;
                console.log("FAILED: no SVG output");
                break;
            }
            if (r >= opts.warmup) times.push(dt);
        }

        if (ok) {
            const { mean, std } = stats(times);
            console.log(`WASM=${mean.toFixed(0)}ms`);
            results.push({ name, wasm_mean: mean, wasm_std: std });
        } else {
            results.push({ name, wasm_mean: null, wasm_std: 0 });
        }
    }

    // Summary table
    console.log();
    console.log("=".repeat(64));
    const nameW = Math.max(...results.map((r) => r.name.length), 7);
    console.log(`  ${"example".padEnd(nameW)}   ${"WASM (ms)".padStart(13)}`);
    console.log("  " + "-".repeat(nameW + 18));
    let total = 0;
    let allOk = true;
    for (const r of results) {
        if (r.wasm_mean === null) {
            allOk = false;
            console.log(`  ${r.name.padEnd(nameW)}   ${"FAILED".padStart(13)}`);
        } else {
            total += r.wasm_mean;
            const col = `${r.wasm_mean.toFixed(1).padStart(8)}±${r.wasm_std.toFixed(1).padStart(5)}`;
            console.log(`  ${r.name.padEnd(nameW)}   ${col}`);
        }
    }
    console.log("  " + "-".repeat(nameW + 18));
    console.log(`  ${"TOTAL".padEnd(nameW)}   ${allOk ? total.toFixed(1).padStart(8) + "      " : "(partial)".padStart(13)}`);
    console.log("=".repeat(64));

    if (opts.json) {
        writeFileSync(
            opts.json,
            JSON.stringify({ runs: opts.runs, warmup: opts.warmup, results }, null, 2),
        );
        console.log(`\nJSON written to ${opts.json}`);
    }

    const failed = results.filter((r) => r.wasm_mean === null).map((r) => r.name);
    if (failed.length) {
        console.error(`\nBuilds failed for: ${failed.join(", ")}`);
        process.exit(1);
    }
}

main();
