import { Diagnostic, DiagnosticSeverity, Range } from "vscode-languageserver";
import { parser } from "@lezer/xml";
import type { SyntaxNode, Tree, TreeCursor } from "@lezer/common";
import { SaxesParser } from "saxes";
import { LineMap } from "../source-object/positions";

/**
 * Phase 0: XML well-formedness diagnostics only. Nothing here knows anything
 * about the PreFigure schema — that is Phase 3, on a separate channel.
 *
 * Two engines, used deliberately:
 *
 *   - `@lezer/xml` is error-tolerant and position-tracking. It keeps parsing
 *     past the first mistake and hands back a tree whose *structure* tells us
 *     which element is unclosed / mismatched, so we can anchor a squiggle on
 *     the exact offending tag (e.g. the `<m>` that is missing its `</m>`).
 *     Its raw error nodes carry no message, so we synthesize the wording from
 *     the node type and surrounding tree.
 *
 *   - `saxes` is a strict streaming parser with excellent, human-worded
 *     messages, and it flags a few lexical problems lezer silently tolerates
 *     (undefined entities, some malformed attribute values). We only fall back
 *     to it when lezer + our own checks find nothing, so its coarser
 *     end-of-token ranges never compete with lezer's precise ones and the same
 *     error is never reported twice.
 */
const SOURCE = "prefigure";

export function validateSyntax(text: string): Diagnostic[] {
    const lines = new LineMap(text);
    const tree = parser.parse(text);

    const structural = structuralDiagnostics(tree, text, lines);
    const duplicateAttributes = duplicateAttributeDiagnostics(tree, text, lines);
    const native = [...structural, ...duplicateAttributes];

    if (native.length > 0) {
        return native.sort(byStart);
    }

    // lezer (which is very tolerant) and our structural checks are clean, but
    // the document may still be malformed in a way only a strict parser
    // catches. Let saxes have the last word in that case.
    return saxesDiagnostics(text, lines).sort(byStart);
}

function structuralDiagnostics(tree: Tree, text: string, lines: LineMap): Diagnostic[] {
    const diagnostics: Diagnostic[] = [];
    const cursor = tree.cursor();
    do {
        switch (cursor.name) {
            case "MissingCloseTag": {
                const open = enclosingOpenTag(cursor.node);
                // When the element's own open tag is itself missing its `>`,
                // the tag case below already reports that precisely; the
                // "unclosed element" fallout lezer also emits is redundant.
                if (open && isTagMissingTerminator(open)) {
                    break;
                }
                const tag = open ? childText(open, "TagName", text) : undefined;
                const range = open
                    ? lines.rangeFromOffsets(open.from, open.to)
                    : lines.rangeFromOffsets(cursor.from, cursor.to);
                diagnostics.push(
                    diag(
                        range,
                        tag
                            ? `Missing closing tag for <${tag}>.`
                            : "Missing closing tag.",
                    ),
                );
                break;
            }
            case "MismatchedCloseTag": {
                const tag = childText(cursor.node, "TagName", text);
                diagnostics.push(
                    diag(
                        lines.rangeFromOffsets(cursor.from, cursor.to),
                        tag
                            ? `Mismatched closing tag </${tag}>.`
                            : "Mismatched closing tag.",
                    ),
                );
                break;
            }
            case "OpenTag":
            case "SelfClosingTag":
            case "CloseTag": {
                // A tag whose terminator (`>` / `/>`) is missing. lezer parses
                // it up to the next `<`, so the only raw signal it leaves is a
                // zero-width error node at the *start of the following tag* —
                // e.g. deleting the `>` of `</annotation>` squiggles the start
                // of the next `</annotations>`, not the tag actually at fault.
                // Catch it here and anchor precisely on the offending tag name.
                if (!isTagMissingTerminator(cursor.node)) {
                    break;
                }
                const name = childText(cursor.node, "TagName", text);
                const isClose = cursor.name === "CloseTag";
                const shown = name
                    ? isClose
                        ? `</${name}>`
                        : `<${name}>`
                    : isClose
                      ? "closing tag"
                      : "tag";
                diagnostics.push(
                    diag(
                        tagNameRange(cursor.node, lines),
                        `Missing ">" to close the ${shown} tag.`,
                    ),
                );
                break;
            }
            default:
                if (cursor.type.isError) {
                    // The zero-width error node lezer drops when a tag is
                    // missing its `>` is already reported precisely by the
                    // tag cases above; don't double up with a vague message
                    // anchored on the wrong tag.
                    if (isTerminatorCollateral(cursor.node)) {
                        break;
                    }
                    diagnostics.push(errorNodeDiagnostic(cursor, text, lines));
                }
        }
    } while (cursor.next());
    return diagnostics;
}

/**
 * lezer's generic error node (`⚠`). When it has width it is usually a stray
 * character (a bare `&` or `<`); when it is zero-width it is usually the
 * parser noticing, at end of input, that an element was never closed.
 */
function errorNodeDiagnostic(cursor: TreeCursor, text: string, lines: LineMap): Diagnostic {
    if (cursor.to > cursor.from) {
        const snippet = text.slice(cursor.from, cursor.to).trim();
        return diag(
            lines.rangeFromOffsets(cursor.from, cursor.to),
            snippet
                ? `Unexpected "${snippet}" in XML — did you mean "&amp;" or "&lt;"?`
                : "Unexpected character in XML.",
        );
    }

    const tag = enclosingOpenTagName(cursor.node, text);
    const range = enclosingOpenTagRange(cursor.node, lines);
    if (range) {
        return diag(range, tag ? `Unclosed tag <${tag}>.` : "Unclosed tag.");
    }
    const from = Math.max(0, cursor.from - 1);
    return diag(lines.rangeFromOffsets(from, cursor.from), "XML syntax error.");
}

/**
 * lezer parses `<p a="1" a="2"/>` without complaint, so we flag repeated
 * attribute names ourselves off the same tree, anchoring precisely on the
 * duplicate name.
 */
function duplicateAttributeDiagnostics(tree: Tree, text: string, lines: LineMap): Diagnostic[] {
    const diagnostics: Diagnostic[] = [];
    const cursor = tree.cursor();
    do {
        if (cursor.name !== "OpenTag" && cursor.name !== "SelfClosingTag") {
            continue;
        }
        const seen = new Set<string>();
        for (let child = cursor.node.firstChild; child; child = child.nextSibling) {
            if (child.name !== "Attribute") {
                continue;
            }
            const nameNode = child.getChild("AttributeName");
            if (!nameNode) {
                continue;
            }
            const name = text.slice(nameNode.from, nameNode.to);
            if (seen.has(name)) {
                diagnostics.push(
                    diag(
                        lines.rangeFromOffsets(nameNode.from, nameNode.to),
                        `Duplicate attribute "${name}".`,
                    ),
                );
            } else {
                seen.add(name);
            }
        }
    } while (cursor.next());
    return diagnostics;
}

function saxesDiagnostics(text: string, lines: LineMap): Diagnostic[] {
    const parserInstance = new SaxesParser({ position: true });
    const diagnostics: Diagnostic[] = [];
    parserInstance.on("error", (error) => {
        const to = parserInstance.position;
        const from = Math.max(0, to - 1);
        diagnostics.push(diag(lines.rangeFromOffsets(from, to), cleanSaxesMessage(error.message)));
    });
    try {
        parserInstance.write(text).close();
    } catch (error) {
        // saxes surfaces most problems through the 'error' handler above and
        // keeps going; a genuinely fatal throw that produced no such event
        // still deserves a diagnostic.
        if (diagnostics.length === 0) {
            const to = parserInstance.position;
            const from = Math.max(0, to - 1);
            const message = error instanceof Error ? error.message : String(error);
            diagnostics.push(diag(lines.rangeFromOffsets(from, to), cleanSaxesMessage(message)));
        }
    }
    return diagnostics;
}

/** Strip saxes' leading "line:col: " prefix — we encode position in the range. */
function cleanSaxesMessage(message: string): string {
    const stripped = message.replace(/^\d+:\d+:\s*/, "").trim();
    if (!stripped) {
        return "XML syntax error.";
    }
    return stripped.charAt(0).toUpperCase() + stripped.slice(1) + (/[.!?]$/.test(stripped) ? "" : ".");
}

/** The grammar node that terminates a given tag, or null for non-tag nodes. */
function terminatorName(nodeName: string): string | null {
    switch (nodeName) {
        case "OpenTag":
        case "CloseTag":
            return "EndTag"; // `>`
        case "SelfClosingTag":
            return "SelfCloseEndTag"; // `/>`
        default:
            return null;
    }
}

/** A tag node whose closing `>` / `/>` is absent (its terminator child). */
function isTagMissingTerminator(node: SyntaxNode): boolean {
    const terminator = terminatorName(node.name);
    return terminator ? !node.getChild(terminator) : false;
}

/**
 * The range covering just `<name` / `</name` of a tag, excluding the trailing
 * whitespace and error node lezer swallows when the terminator is missing, so
 * the squiggle sits on the tag at fault rather than bleeding onto the next line.
 */
function tagNameRange(node: SyntaxNode, lines: LineMap): Range {
    let end = node.from;
    for (let child = node.firstChild; child; child = child.nextSibling) {
        if (child.type.isError) {
            continue;
        }
        if (child.to > end) {
            end = child.to;
        }
    }
    if (end <= node.from) {
        end = node.to;
    }
    return lines.rangeFromOffsets(node.from, end);
}

/**
 * True when an error node is merely the fallout of a tag that is already being
 * reported for a missing terminator — either the error node sits directly
 * inside that tag, or it is the zero-width "unclosed element" marker for an
 * element whose own open tag is the malformed one.
 */
function isTerminatorCollateral(node: SyntaxNode): boolean {
    const parent = node.parent;
    if (!parent) {
        return false;
    }
    if (isTagMissingTerminator(parent)) {
        return true;
    }
    if (parent.name === "Element") {
        const open = parent.getChild("OpenTag");
        if (open && isTagMissingTerminator(open)) {
            return true;
        }
    }
    return false;
}

function enclosingOpenTagRange(node: SyntaxNode, lines: LineMap): Range | undefined {
    const open = enclosingOpenTag(node);
    return open ? lines.rangeFromOffsets(open.from, open.to) : undefined;
}

function enclosingOpenTagName(node: SyntaxNode, text: string): string | undefined {
    const open = enclosingOpenTag(node);
    return open ? childText(open, "TagName", text) : undefined;
}

/** The `OpenTag` of the `Element` that owns `node`, if any. */
function enclosingOpenTag(node: SyntaxNode): SyntaxNode | null {
    const element = node.parent;
    return element ? element.getChild("OpenTag") : null;
}

function childText(node: SyntaxNode, childName: string, text: string): string | undefined {
    const child = node.getChild(childName);
    return child ? text.slice(child.from, child.to) : undefined;
}

function diag(range: Range, message: string): Diagnostic {
    return { range, message, severity: DiagnosticSeverity.Error, source: SOURCE };
}

function byStart(a: Diagnostic, b: Diagnostic): number {
    return (
        a.range.start.line - b.range.start.line ||
        a.range.start.character - b.range.start.character
    );
}
