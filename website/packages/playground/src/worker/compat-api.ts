import { toBraille } from "./liblouis";

// @ts-ignore
import { setupEngine, toSpeech } from "speech-rule-engine";
import { mathjax } from "mathjax-full/js/mathjax.js";
import { TeX } from "mathjax-full/js/input/tex.js";
import { SVG } from "mathjax-full/js/output/svg.js";
import { liteAdaptor } from "mathjax-full/js/adaptors/liteAdaptor.js";
import { RegisterHTMLHandler } from "mathjax-full/js/handlers/html.js";
import { SerializedMmlVisitor } from "mathjax-full/js/core/MmlTree/SerializedMmlVisitor";
import { MathDocument } from "mathjax-full/js/core/MathDocument";

/**
 * This is the API used by PreFigure when running in the browser. It implements the necessary
 * functions for Prefigure's abstract classes.
 */
export class PrefigBrowserApi {
    offscreenCanvas: OffscreenCanvas | null = null;
    ctx: OffscreenCanvasRenderingContext2D | null = null;
    mathDocumentSvg: MathDocument<any, any, any> | null = null;
    mathDocumentMml: MathDocument<any, any, any> | null = null;
    initFinished: Promise<void> = Promise.resolve();
    adaptor = liteAdaptor();

    constructor() {
        let resolve: Function;
        this.initFinished = new Promise((r) => {
            resolve = r;
        });

        // Everything that needs asynchronous loading should happen in this block.
        (async () => {
            // Needed for MathJax
            RegisterHTMLHandler(this.adaptor);
            const tex = new TeX();
            const svg = new SVG();
            this.mathDocumentSvg = mathjax.document("", {
                InputJax: tex,
                OutputJax: svg,
            });
            this.mathDocumentMml = mathjax.document("", {
                InputJax: tex,
            });

            await setupEngine({
                locale: "nemeth",
                modality: "braille",
            });

            // Don't forget to call this otherwise the engine will never be ready!
            resolve!();
        })();
    }

    /**
     * Measure the extents of typeset text.
     */
    measure_text(text: string, font_string: string) {
        if (!this.offscreenCanvas) {
            this.offscreenCanvas = new OffscreenCanvas(200, 200);
        }
        if (!this.ctx) {
            this.ctx = this.offscreenCanvas.getContext("2d");
            if (!this.ctx) {
                throw new Error("Failed to create canvas context");
            }
        }
        // XXX replace this with proper data from `font_data`
        this.ctx.font = font_string;

        const tm = this.ctx.measureText(text);
        console.log("Measured text", text, tm);
        return [
            tm.width,
            tm.actualBoundingBoxAscent,
            tm.actualBoundingBoxDescent,
        ];
    }

    /**
     * Translate text to Braille.
     */
    translate_text(text: string, _typeform: number[]): string {
        return toBraille(text, { mode: "unicode", contracted: true });
    }

    processMath(expression: string): string {
        if (!this.mathDocumentSvg) {
            throw new Error(
                "PrefigBrowserApi not correctly initialized (missing mathDocumentSvg)",
            );
        }

        const node = this.mathDocumentSvg.convert(expression, {
            display: false,
        });
        const result = this.adaptor.outerHTML(node);
        return result;
    }

    processBraille(expression: string): string {
        if (!this.mathDocumentMml) {
            throw new Error(
                "PrefigBrowserApi not correctly initialized (missing mathDocumentMml)",
            );
        }

        const mathNode = this.mathDocumentMml.convert(expression, {
            display: true, // Set to false for inline math
            end: "mathml", // Produce MathML output
        });

        const visitor = new SerializedMmlVisitor();
        // A string containing a MathML version of the math
        const mml = visitor.visitTree(mathNode);

        // A string containing braille version of the MathML
        const braille = toSpeech(mml);

        return braille;
    }
}

export const prefigBrowserApi = new PrefigBrowserApi();

(globalThis as any).prefigBrowserApi = prefigBrowserApi;
