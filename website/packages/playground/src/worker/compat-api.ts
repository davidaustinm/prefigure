/**
 * This is the API used by PreFigure when running in the browser. It implements the necessary
 * functions for Prefigure's abstract classes.
 */

//import { translateString } from "liblouis";

export class PrefigBrowserApi {
    offscreenCanvas: OffscreenCanvas | null = null;
    ctx: OffscreenCanvasRenderingContext2D | null = null;

    measure_text(text: string, _font_data?: unknown) {
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
        this.ctx.font = "14px sans";

        const tm = this.ctx.measureText(text);
        console.log("Measured text", text, tm);
        return [
            tm.width,
            tm.actualBoundingBoxAscent,
            tm.actualBoundingBoxDescent,
        ];
    }

    //translate_text(text: string, typeform: number[]): string {
    //    //const tables = ["tables/braille-patterns.cti", "tables/en-us-g2.ctb"];
    //    const tables = "tables/en-us-g2.ctb";
    //    try {
    //        //          const unicode_result = translateString(tables, text);
    //        const unicode_result = "";
    //        console.log("Translated string", unicode_result);
    //        return unicode_result;
    //    } catch (error) {
    //        console.log("Error translating text:", error);
    //        return "";
    //    }
    //}
}

export const prefigBrowserApi = new PrefigBrowserApi();

(globalThis as any).prefigBrowserApi = prefigBrowserApi;
