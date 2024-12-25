import { toBraille } from "./liblouis";

import { mathjax } from 'mathjax-full/js/mathjax.js';
import { TeX } from 'mathjax-full/js/input/tex.js';
import { SVG } from 'mathjax-full/js/output/svg.js';
import { liteAdaptor } from 'mathjax-full/js/adaptors/liteAdaptor.js';
import { RegisterHTMLHandler } from 'mathjax-full/js/handlers/html.js';
import { AllPackages } from 'mathjax-full/js/input/tex/AllPackages.js';
import { CHTML } from 'mathjax-full/js/output/chtml.js';

// import { sre } from 'speech-rule-engine';


/**
 * This is the API used by PreFigure when running in the browser. It implements the necessary
 * functions for Prefigure's abstract classes.
 */
export class PrefigBrowserApi {
    offscreenCanvas: OffscreenCanvas | null = null;
    ctx: OffscreenCanvasRenderingContext2D | null = null;

    constructor() {
        /*
        sre.setupEngine({
            locale: 'nemeth',
            domain: 'nemeth',
            modality: 'braille',
        });
        */
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
        const adaptor = liteAdaptor();
        RegisterHTMLHandler(adaptor);

        const tex = new TeX();
        const svg = new SVG();
        const mj = mathjax.document('', { InputJax: tex, OutputJax: svg });

        const node = mj.convert(expression, { display: false });
        const result = adaptor.outerHTML(node);
        return result;
    }

    processBraille(expression: string): string {
        return expression;
        /*
        const adaptor = liteAdaptor();
        RegisterHTMLHandler(adaptor);

        const tex = new TeX({ packages: AllPackages });
        const chtml = new CHTML();
        const mj = mathjax.document('', { InputJax: tex, OutputJax: chtml });

        // Convert the expression to MathML
        const mathml = adaptor.innerHTML(mj.convert(expression, { display: false, format: 'MathML' }));

        // Use SRE to generate Braille output
        const braille = mathjax.startup.sre.System.toSpeech(mathml, {
          locale: 'nemeth',
          domain: 'nemeth',
          modality: 'braille',
        });
        console.log(braille);

        return braille;
        */
    }

}

export const prefigBrowserApi = new PrefigBrowserApi();

(globalThis as any).prefigBrowserApi = prefigBrowserApi;
