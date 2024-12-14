import { describe, expect, it } from "vitest";
import { toBraille } from "../src/worker/liblouis";

describe("Convert strings to Braille", () => {
    it("translateString converts to condensed Braille", () => {
        const result = toBraille("Hello, World!", {
            contracted: true,
            mode: "brf",
        });
        expect(result).toBe(",hello1 ,_w6");
    });
    it("translateString converts to condensed unicode Braille", () => {
        const result = toBraille("Hello, World!", {
            contracted: true,
            mode: "unicode",
        });
        // Note: The space character is the Braille space character in the output, not a regular space.
        expect(result).toBe("⠠⠓⠑⠇⠇⠕⠂ ⠠⠸⠺⠖".replaceAll(" ", "⠀"));
    });
    it("translateString converts to uncondensed Braille", () => {
        const result = toBraille("Hello, World!", {
            contracted: false,
            mode: "brf",
        });
        expect(result).toBe(",hello1 ,world6");
    });
    it("translateString converts to uncondensed unicode Braille", () => {
        const result = toBraille("Hello, World!", {
            contracted: false,
            mode: "unicode",
        });
        // Note: The space character is the Braille space character in the output, not a regular space.
        expect(result).toBe("⠠⠓⠑⠇⠇⠕⠂ ⠠⠺⠕⠗⠇⠙⠖".replaceAll(" ", "⠀"));
    });
});
