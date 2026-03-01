import Editor from "@monaco-editor/react";
import React, { useEffect, useState } from "react";
import { useStoreState, useStoreActions } from "../state";
import { convert } from "@naman22khater/data-converter";
import { Button, ButtonGroup, Nav, ToggleButton } from "react-bootstrap";
import { Download, Trash } from "react-bootstrap-icons";
import { saveAs } from "file-saver";

type EditingMode = "xml" | "yaml";

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
