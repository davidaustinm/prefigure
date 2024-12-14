import { describe, expect, it } from "vitest";
import { translateString } from "../src/worker/liblouis/easy-api";

describe("Convert strings to Braille", () => {
    it("translateString converts to condensed Braille", () => {
        const result = translateString("en-ueb-g2.ctb", "Hello, World!");
        expect(result).toBe(",hello1 ,_w6");
        //       expect(result).toBe("⠨⠓⠨⠑⠨⠋⠨⠇⠨⠇⠨⠕⠨⠳⠨⠕⠨⠇⠨⠙⠨⠥");
    });
    it("translateString converts to uncondensed Braille", () => {
        const result = translateString("en-ueb-g1.ctb", "Hello, World!");
        expect(result).toBe(",hello1 ,world6");
        //       expect(result).toBe("⠨⠓⠨⠑⠨⠋⠨⠇⠨⠇⠨⠕⠨⠳⠨⠕⠨⠇⠨⠙⠨⠥");
    });
});
