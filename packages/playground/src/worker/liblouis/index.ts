import { translateString } from "./easy-api";

const DEFAULT_OPTIONS = {
    mode: "brf",
    compact: true,
} as const;

/**
 * Translate a string to braille, returning either a BRF or Unicode string.
 */
export function toBraille(
    text: string,
    options?: {
        /**
         * Whether to use BRF (braille ready format) or Unicode braille.
         */
        mode?: "brf" | "unicode";
        /**
         * Whether to use contracted braille output or not (E.g., `the` gets abbreviated to `!`).
         */
        contracted?: boolean;
    },
): string {
    const { mode, contracted: compact } = { ...DEFAULT_OPTIONS, ...(options || {}) };

    const table = compact ? "en-ueb-g2.ctb" : "en-ueb-g1.ctb";

    const result = translateString(table, text);
    if (result == null) {
        console.warn("Failed to translate text to braille:", text, {
            mode,
            compact,
        });
    }

    if (result == null) {
        return "";
    }

    if (mode === "brf") {
        return result;
    }

    return brfToUnicode(result);
}

const BRAILLE_ASCII_TO_UNICODE = [
    "⠀",
    "⠮",
    "⠐",
    "⠼",
    "⠫",
    "⠩",
    "⠯",
    "⠄",
    "⠷",
    "⠾",
    "⠡",
    "⠬",
    "⠠",
    "⠤",
    "⠨",
    "⠌",
    "⠴",
    "⠂",
    "⠆",
    "⠒",
    "⠲",
    "⠢",
    "⠖",
    "⠶",
    "⠦",
    "⠔",
    "⠱",
    "⠰",
    "⠣",
    "⠿",
    "⠜",
    "⠹",
    "⠈",
    "⠁",
    "⠃",
    "⠉",
    "⠙",
    "⠑",
    "⠋",
    "⠛",
    "⠓",
    "⠊",
    "⠚",
    "⠅",
    "⠇",
    "⠍",
    "⠝",
    "⠕",
    "⠏",
    "⠟",
    "⠗",
    "⠎",
    "⠞",
    "⠥",
    "⠧",
    "⠺",
    "⠭",
    "⠽",
    "⠵",
    "⠪",
    "⠳",
    "⠻",
    "⠘",
    "⠸",
    "⠈",
    "⠁",
    "⠃",
    "⠉",
    "⠙",
    "⠑",
    "⠋",
    "⠛",
    "⠓",
    "⠊",
    "⠚",
    "⠅",
    "⠇",
    "⠍",
    "⠝",
    "⠕",
    "⠏",
    "⠟",
    "⠗",
    "⠎",
    "⠞",
    "⠥",
    "⠧",
    "⠺",
    "⠭",
    "⠽",
    "⠵",
    "⠪",
    "⠳",
    "⠻",
    "⠘",
    "⠸",
];

/**
 * Translate a BRF string to Unicode.
 */
export function brfToUnicode(brf: string): string {
    // If we are less than ASCII code 32, we are a control character
    // that is preserved. Otherwise, we look up our value in the translation table.
    return brf
        .split("")
        .map((c) => {
            const code = c.charCodeAt(0);
            if (code < 32) {
                return c;
            }
            return BRAILLE_ASCII_TO_UNICODE[code - 32];
        })
        .join("");
}
