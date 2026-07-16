import Editor from "@monaco-editor/react";
import React, { useEffect, useState } from "react";
import { useStoreState, useStoreActions } from "../state";
import { convert } from "@naman22khater/data-converter";
import { Alert, Button, ButtonGroup, Nav, ToggleButton } from "react-bootstrap";
import { Download, Trash } from "react-bootstrap-icons";
import { saveAs } from "file-saver";

type EditingMode = "xml" | "yaml";

/**
 * Pick out the one line worth showing by default from a raw error string.
 *
 * Compiling runs Python inside Pyodide, so a failure often arrives as a full
 * Python traceback (itself wrapped by Pyodide's own call stack). The last
 * line of that traceback is the actual `ExceptionType: message`; the rest is
 * interpreter plumbing that isn't useful at a glance. Anything that isn't a
 * traceback (e.g. a plain JS Error) is already short, so it's shown as-is.
 */
function summarizeError(raw: string): string {
    const trimmed = raw.trim();
    if (!trimmed.includes("Traceback (most recent call last):")) {
        return trimmed;
    }
    const lines = trimmed
        .split("\n")
        .map((line) => line.trim())
        .filter((line) => line.length > 0);
    return lines[lines.length - 1] || trimmed;
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
 * A source editor component for XML/YAML code using CodeMirror 6 directly.
 * This component allows the user to type PreFigure code.
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

    return (
        <div className="panel-frame">
            <div className="editor-buffer">
                <Editor
                    width="100%"
                    height="100%"
                    language={editingMode}
                    value={contentXmlOrYaml}
                    onMount={(editor, monaco) => {
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
