/**
 * Encodes a source string for use in the `s` query parameter, e.g.
 * `?s=<encoded>`. Produces unpadded, URL-safe base64 so the result never
 * needs percent-encoding.
 */
export function encodeSourceForQueryParam(source: string): string {
    const bytes = new TextEncoder().encode(source);
    let binary = "";
    for (const byte of bytes) {
        binary += String.fromCharCode(byte);
    }
    return btoa(binary)
        .replace(/\+/g, "-")
        .replace(/\//g, "_")
        .replace(/=+$/, "");
}

/**
 * Reads the `s` query parameter, if present, and decodes it as a base64
 * (standard or URL-safe) encoded UTF-8 string. Used to pre-populate the
 * editor from a shared link, e.g. `?s=<base64>`.
 */
export function getSourceFromQueryParam(): string | undefined {
    const raw = new URLSearchParams(window.location.search).get("s");
    if (!raw) {
        return undefined;
    }
    try {
        // Undo the `+` -> ` ` substitution `URLSearchParams` applies, and
        // accept the URL-safe base64 alphabet (`-`/`_` instead of `+`/`/`).
        const base64 = raw
            .replace(/ /g, "+")
            .replace(/-/g, "+")
            .replace(/_/g, "/");
        const padded = base64.padEnd(
            base64.length + ((4 - (base64.length % 4)) % 4),
            "=",
        );
        const binary = atob(padded);
        const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
        return new TextDecoder().decode(bytes);
    } catch {
        console.warn("Ignoring malformed `s` query parameter");
        return undefined;
    }
}
