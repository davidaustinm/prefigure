import { PluginOption, defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { viteStaticCopy } from "vite-plugin-static-copy";

const PYODIDE_EXCLUDE = [
    "!**/*.{md,html}",
    "!**/*.d.ts",
    "!**/*.whl",
    "!**/node_modules",
];

export default defineConfig({
    optimizeDeps: { exclude: ["pyodide"] },
    plugins: [react(), vitePluginPyodide()],
    base: "./",
    worker: {
        format: "es",
    },
    build: {
        rollupOptions: {
            output: {
                inlineDynamicImports: true,
            },
        },
    },
});

function vitePluginPyodide(): PluginOption {
    const pyodideDir = dirname(fileURLToPath(import.meta.resolve("pyodide")));
    return viteStaticCopy({
        targets: [
            // Copy the assets needed from the pyodide node_modules
            {
                src: [join(pyodideDir, "*")].concat(PYODIDE_EXCLUDE),
                dest: "assets/pyodide",
            },
            // Copy the wheels from pyodide_packages
            {
                src: [
                    join("pyodide_packages", "**", "*.whl"),
                    join("pyodide_packages", "**", "*.zip"),
                ],
                dest: "assets/pyodide",
            },
            // Copy our currently build PreFigure wheel
            {
                src: [join("../../../dist", "prefig-0.*.*-py3-none-any.whl")],
                dest: "assets/pyodide",
            },
        ],
    });
}
