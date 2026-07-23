import Editor, { Monaco } from "@monaco-editor/react";
import React, { useCallback, useEffect, useRef, useState } from "react";
import type { editor as MonacoEditor } from "monaco-editor";
import { useStoreState, useStoreActions } from "../state";
import { convert } from "@naman22khater/data-converter";
import { Alert, Button, ButtonGroup, Nav, ToggleButton } from "react-bootstrap";
import { Download, Trash } from "react-bootstrap-icons";
import { saveAs } from "file-saver";
import { Diagnostic, PrefigureLspClient } from "../lsp-client/client";
import {
    compileErrorToDiagnostics,
    summarizeError,
} from "../lsp-client/compile-error";

type EditingMode = "xml" | "yaml";

// Owner name for the markers this component pushes onto the Monaco model.
// Namespacing them means clearing our markers never disturbs anyone else's.
const LSP_MARKER_OWNER = "prefigure-lsp";

/**
 * Map an LSP diagnostic severity (1=Error … 4=Hint) to a Monaco marker
 * severity. Everything in Phase 0/1 is an error, but do it properly so
 * warnings/hints render correctly once later phases emit them.
 */
function toMarkerSeverity(monaco: Monaco, severity: number | undefined): number {
    switch (severity) {
        case 2:
            return monaco.MarkerSeverity.Warning;
        case 3:
            return monaco.MarkerSeverity.Info;
        case 4:
            return monaco.MarkerSeverity.Hint;
        default:
            return monaco.MarkerSeverity.Error;
    }
}

const CONTENT_AFTER_CLEAR = `<diagram dimensions="(300, 300)" margins="5">

</diagram>`;

/**
 * Translate XML to Yaml.
 *
 * @param {string} source The XML source string.
 * @returns {string} The corresponding Yaml string.
 */
function xmlToYaml(source: string): string {
    return convert(source, { from: "xml", to: "yaml" }).output;
}

/**
 * Translate Yaml to XML.
 *
 * @param {string} source The Yaml source string.
 * @returns {string} The corresponding XML string.
 */
function yamlToXml(source: string): string {
    return convert(source, {
        from: "yaml",
        to: "xml",
        serializeOptions: {
            declaration: false,
            rootElement: "",
        },
    }).output;
}

// Counter to avoid errors during initialization.
let init = 2;

/**
 * A source editor component for XML/YAML code, built on the Monaco editor.
 * This component allows the user to type PreFigure code, and (in XML mode)
 * wires it to the PreFigure language server for live diagnostics.
 */
export function SourceEditor() {
    const sourceXml = useStoreState((state) => state.source);
    const setSourceXml = useStoreActions((actions) => actions.setSource);
    const toolbarRef = React.useRef<HTMLDivElement>(null);
    const error = useStoreState((state) => state.errorState);
    const setErrorState = useStoreActions((actions) => actions.setErrorState);
    const [showErrorDetails, setShowErrorDetails] = useState(false);

    // The editor may be working in XML or YAML. However, the source is always stored in XML, so `contentXmlOrYaml`
    // effectively shadows `sourceXml`, but could be a YAML string which gets translated to XML on the fly.
    const [contentXmlOrYaml, setContent] = useState<string>(sourceXml);
    const [editingMode, setEditingMode] = useState<EditingMode>("xml");

    // Translate source when language changes
    useEffect(() => {
        try {
            if (!sourceXml.trim()) return;
            if (editingMode === "yaml") {
                setContent(xmlToYaml(contentXmlOrYaml));
            } else {
                if (init) {
                    init--;
                    return;
                }
                setContent(yamlToXml(contentXmlOrYaml));
            }
        } catch {
            // Ignore conversion errors (e.g. invalid syntax while editing)
            // These should not happen as the selection box will be grayed out.
        }
    }, [editingMode]);

    // Collapse the details section whenever the error changes (including
    // when it's cleared), so a new failure doesn't inherit the old expanded
    // state and a dismissed error doesn't leave stale details lying around.
    useEffect(() => {
        setShowErrorDetails(false);
    }, [error]);

    // --- Language-server wiring -------------------------------------------
    // A thin client to the PreFigure LSP running in a Web Worker. It validates
    // XML well-formedness (Phase 0) as you type and, via a second channel, we
    // feed it the real compile errors. Diagnostics are painted as Monaco
    // markers. The LSP only understands XML, so it is active in XML mode only.
    const lspClientRef = useRef<PrefigureLspClient | null>(null);
    const monacoRef = useRef<Monaco | null>(null);
    const editorRef = useRef<MonacoEditor.IStandaloneCodeEditor | null>(null);
    const latestDiagnosticsRef = useRef<Diagnostic[]>([]);
    // Read the current mode/content from the diagnostics callback without
    // re-subscribing it on every keystroke.
    const editingModeRef = useRef<EditingMode>(editingMode);
    editingModeRef.current = editingMode;
    const contentRef = useRef<string>(contentXmlOrYaml);
    contentRef.current = contentXmlOrYaml;
    // The editor text a compile error was produced against. A compile always
    // runs on the current text, so when `error` changes the two match; the
    // moment the user edits, the error's line/column no longer line up, so we
    // drop the compile squiggles until the next compile re-establishes them.
    const errorContentRef = useRef<string>(contentXmlOrYaml);
    // Whether the compile channel currently holds any squiggle — lets the edit
    // effect clear exactly once instead of re-pushing empty on every keystroke.
    const compileDiagnosticsActiveRef = useRef<boolean>(false);
    // Debounce timer for pushing document changes to the LSP worker.
    const syncTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

    const applyMarkers = useCallback(() => {
        const monaco = monacoRef.current;
        const model = editorRef.current?.getModel();
        if (!monaco || !model) {
            return;
        }
        if (editingModeRef.current !== "xml") {
            monaco.editor.setModelMarkers(model, LSP_MARKER_OWNER, []);
            return;
        }
        const markers = latestDiagnosticsRef.current.map((d) => ({
            severity: toMarkerSeverity(monaco, d.severity),
            // `message` is `string | MarkupContent` in the protocol types; our
            // server only ever sends strings, but coerce to be safe.
            message: typeof d.message === "string" ? d.message : d.message.value,
            source: d.source,
            startLineNumber: d.range.start.line + 1,
            startColumn: d.range.start.character + 1,
            endLineNumber: d.range.end.line + 1,
            endColumn: d.range.end.character + 1,
        }));
        monaco.editor.setModelMarkers(model, LSP_MARKER_OWNER, markers);
    }, []);

    // Spawn the LSP worker once for the lifetime of the editor.
    useEffect(() => {
        const client = new PrefigureLspClient((diagnostics) => {
            latestDiagnosticsRef.current = diagnostics;
            applyMarkers();
        });
        lspClientRef.current = client;
        return () => {
            client.dispose();
            lspClientRef.current = null;
        };
    }, [applyMarkers]);

    // Keep the server's document in sync with the editor (XML mode only),
    // debounced so a burst of keystrokes coalesces into one parse instead of
    // one per character. In YAML mode there is no XML to validate, so clear
    // our markers immediately.
    useEffect(() => {
        if (editingMode !== "xml") {
            if (syncTimerRef.current !== null) {
                clearTimeout(syncTimerRef.current);
                syncTimerRef.current = null;
            }
            const monaco = monacoRef.current;
            const model = editorRef.current?.getModel();
            if (monaco && model) {
                monaco.editor.setModelMarkers(model, LSP_MARKER_OWNER, []);
            }
            return;
        }
        const text = contentXmlOrYaml;
        syncTimerRef.current = setTimeout(() => {
            syncTimerRef.current = null;
            void lspClientRef.current?.syncDocument(text);
        }, 200);
        return () => {
            if (syncTimerRef.current !== null) {
                clearTimeout(syncTimerRef.current);
                syncTimerRef.current = null;
            }
        };
    }, [contentXmlOrYaml, editingMode]);

    // Bridge compile errors into the LSP's second diagnostic channel. Runs
    // when the error changes; a compile always runs on the current text, so
    // the error's positions line up with what's on screen right now. Record
    // that text so the edit effect below can tell when they stop lining up.
    // Well-formedness errors are dropped here because the native syntax pass
    // already squiggles them precisely.
    useEffect(() => {
        const client = lspClientRef.current;
        if (!client) {
            return;
        }
        if (editingMode !== "xml") {
            compileDiagnosticsActiveRef.current = false;
            void client.setAdditionalDiagnostics([]);
            return;
        }
        const diagnostics = compileErrorToDiagnostics(error, contentRef.current);
        errorContentRef.current = contentRef.current;
        compileDiagnosticsActiveRef.current = diagnostics.length > 0;
        void client.setAdditionalDiagnostics(diagnostics);
    }, [error, editingMode]);

    // Compile diagnostics carry fixed line/column positions from the last
    // compile. Once the user edits, those positions drift from the text, so
    // clear them right away rather than leaving a squiggle stranded on the
    // wrong line. The next compile (debounced in the model) re-pushes fresh
    // ones via the effect above.
    useEffect(() => {
        if (editingMode !== "xml") {
            return;
        }
        if (
            compileDiagnosticsActiveRef.current &&
            contentXmlOrYaml !== errorContentRef.current
        ) {
            compileDiagnosticsActiveRef.current = false;
            void lspClientRef.current?.setAdditionalDiagnostics([]);
        }
    }, [contentXmlOrYaml, editingMode]);

    return (
        <div className="panel-frame">
            <div className="editor-buffer">
                <Editor
                    width="100%"
                    height="100%"
                    language={editingMode}
                    value={contentXmlOrYaml}
                    onMount={(editor, monaco) => {
                        editorRef.current = editor;
                        monacoRef.current = monaco;
                        // Paint any diagnostics that arrived before the editor
                        // finished mounting.
                        applyMarkers();
                        editor.addAction({
                            id: "return-focus",
                            label: "Return focus",
                            keybindings: [monaco.KeyCode.Escape],
                            run: () => {
                                const activeElement =
                                    document.activeElement instanceof
                                    HTMLElement
                                        ? document.activeElement
                                        : editor.getDomNode();
                                activeElement?.blur();
                                if (toolbarRef.current) {
                                    toolbarRef.current.focus();
                                }
                            },
                        });
                    }}
                    options={{
                        minimap: { enabled: false },
                        lineNumbers: "off",
                    }}
                    onChange={(value) => {
                        if (value !== undefined) {
                            setSourceXml(
                                editingMode === "xml"
                                    ? value
                                    : yamlToXml(value),
                            );
                            setContent(value);
                        }
                    }}
                />
            </div>
            {error && (
                <Alert
                    variant="danger"
                    onClose={() => setErrorState("")}
                    dismissible
                    className="compile-error-alert"
                >
                    <div className="compile-error-summary">
                        <strong>Compile error:</strong> {summarizeError(error)}
                    </div>
                    {summarizeError(error) !== error.trim() && (
                        <Button
                            size="sm"
                            variant="link"
                            className="compile-error-toggle"
                            onClick={() =>
                                setShowErrorDetails((shown) => !shown)
                            }
                        >
                            {showErrorDetails
                                ? "Hide details"
                                : "Show details"}
                        </Button>
                    )}
                    {showErrorDetails && (
                        <pre className="compile-error-details">{error}</pre>
                    )}
                </Alert>
            )}
            <Nav className="panel-toolbar" ref={toolbarRef} tabIndex={0}>
                <Button
                    size="sm"
                    variant="warning"
                    onClick={() => {
                        setSourceXml(CONTENT_AFTER_CLEAR);
                        setContent(
                            editingMode === "xml"
                                ? CONTENT_AFTER_CLEAR
                                : xmlToYaml(CONTENT_AFTER_CLEAR),
                        );
                    }}
                >
                    <Trash /> Clear
                </Button>
                <ButtonGroup>
                    <ToggleButton
                        id="toggle-xml"
                        value={1}
                        checked={editingMode === "xml"}
                        active={editingMode === "xml"}
                        variant={
                            editingMode === "xml"
                                ? "primary"
                                : "outline-secondary"
                        }
                        onClick={() => setEditingMode("xml")}
                        title="Edit XML source code"
                    >
                        XML
                    </ToggleButton>
                    <ToggleButton
                        id="toggle-yaml"
                        value={2}
                        checked={editingMode === "yaml"}
                        active={editingMode === "yaml"}
                        variant={
                            editingMode === "yaml"
                                ? "primary"
                                : "outline-secondary"
                        }
                        onClick={() => setEditingMode("yaml")}
                        title="Edit YAML source code"
                    >
                        YAML
                    </ToggleButton>
                </ButtonGroup>
                <Button
                    size="sm"
                    onClick={() => {
                        const blob = new Blob([sourceXml], {
                            type: "text/plain",
                        });
                        saveAs(blob, "figure.xml");
                    }}
                    title="Download the XML source code"
                >
                    <Download /> Download Source
                </Button>
            </Nav>
        </div>
    );
}
