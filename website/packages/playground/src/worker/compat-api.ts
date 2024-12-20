import { toBraille } from "./liblouis";

/**
 * This is the API used by PreFigure when running in the browser. It implements the necessary
 * functions for Prefigure's abstract classes.
 */
export class PrefigBrowserApi {
    offscreenCanvas: OffscreenCanvas | null = null;
    ctx: OffscreenCanvasRenderingContext2D | null = null;

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
}

export const prefigBrowserApi = new PrefigBrowserApi();

(globalThis as any).prefigBrowserApi = prefigBrowserApi;
