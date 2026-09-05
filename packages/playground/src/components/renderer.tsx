import React from "react";
import { useStoreActions, useStoreState } from "../state";
import {
    Button,
    ButtonGroup,
    Dropdown,
    Nav,
    Spinner,
    SplitButton,
    ToggleButton,
} from "react-bootstrap";
import { Download } from "react-bootstrap-icons";
import { saveAs } from "file-saver";
import * as diagcess from "diagcess";


/**
 * Assembles a code snippet that contains the SVG and the annotations in a single HTML element.
 *
 * @param {string} annotations The annotations in DiagCess format.
 * @param {string} svg The SVG code to be displayed.
 * @returns {string} The assembled code snippet as a string.
 */
function assembleCodeSnippet(svg: string, annotations: string): string {
  let result = '<div class="ChemAccess-element">\n';
  result += `  <div class="svg">\n    ${svg}\n  </div>\n`;
  result += `  <div class="cml">\n    ${annotations}\n  </div>\n`;
  result += '</div>\n';
  return result;
}

/**
 * @returns {string} The script snippet that initializes the DiagCess library,
 * sets up the necessary event listeners, and sets the default language.
 */
function scriptSnippet(): string {
  let result = '<div class="diagcess-script-container">\n';
  result += '  <script type="text/javascript" src="https://cdn.jsdelivr.net/npm/diagcess@latest/dist/diagcess.js"></script>\n';
  result += '  <script type="text/javascript">\n';
  result += '  document.addEventListener("DOMContentLoaded", function () {\n';
  result += '    diagcess.Base.init();\n';
  result += '    diagcess.Config.VOICE_LANG = "en";\n';
  result += '  })\n';
  result += '  </script>\n';
  result += '</div>';
  return result;
}

/**
 * Renders and displays the currently active PreFigure source code.
 */
export function Renderer() {
    const loadPyodide = useStoreActions((actions) => actions.loadPyodide);
    const status = useStoreState((state) => state.status);
    const compiledSource = useStoreState((state) => state.compiledSource);
    const annotations = useStoreState((state) => state.annotations);
    const compile = useStoreActions((actions) => actions.compile);
    const mode = useStoreState((state) => state.compileMode);
    const setMode = useStoreActions((actions) => actions.setCompileMode);
    const needsCompile = useStoreState((state) => state.needsCompile);

    // We add `viewBox="..."` and `preserveAspectRatio="xMidYMid meet"` to the SVG to make sure it scales correctly
    // in our display region.
    const sourceForDisplay = React.useMemo(() => {
        if (typeof compiledSource !== "string") {
            console.warn(
                "`compiledSource` is not available. This means the python code failed to compile the source code. Expected a string, got:",
                compiledSource,
            );
            return `<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%">
                <text x="50%" y="50%" text-anchor="middle" dominant-baseline="middle" font-size="24" fill="black">
                    No diagrams found in PreFigure source
                </text>
            </svg>`;
        }
        if (!compiledSource.startsWith("<svg")) {
            return compiledSource;
        }

        // Create a new XML parser
        const parser = new DOMParser();
        // Parse the SVG
        const svg = parser.parseFromString(compiledSource, "image/svg+xml");
        // Get the width and height attributes from the root SVG element
        const width = svg.documentElement.getAttribute("width");
        const height = svg.documentElement.getAttribute("height");
        // Set the viewBox attribute to match the width and height
        svg.documentElement.setAttribute("viewBox", `0 0 ${width} ${height}`);
        // Set the preserveAspectRatio attribute to make the SVG scale correctly
        svg.documentElement.setAttribute(
            "preserveAspectRatio",
            "xMidYMid meet",
        );
        // Remove the width and height attributes
        svg.documentElement.removeAttribute("width");
        svg.documentElement.removeAttribute("height");

        // Serialize the new SVG
        return new XMLSerializer().serializeToString(svg.documentElement);
    }, [compiledSource]);

    React.useEffect(() => {
        loadPyodide();
    }, []);

    if (status === "loadingPyodide" || status === "loadingWasm") {
        return (
            <div className="loading">
                <Spinner animation="border" style={{ marginRight: "5px" }} />{" "}
                {status === "loadingWasm"
                    ? "Loading WebAssembly engine..."
                    : "Loading Pyodide..."}
            </div>
        );
    }

    return (
        <div className="panel-frame">
            <div className="render-buffer">
                <div className="render-content">
                    {sourceForDisplay.startsWith("<svg") ? (
                        <AnnotateSvg
                            svg={sourceForDisplay}
                            annotations={annotations}
                        />
                    ) : (
                        compiledSource
                    )}
                </div>
            </div>
            <Nav className="panel-toolbar">
                <Button
                    variant="success"
                    style={{ flexBasis: 100 }}
                    onClick={() => compile()}
                    title={`Compile the PreFigure source code to an svg.${needsCompile ? " The source has changed since last being compiled." : ""}`}
                >
                    Compile{needsCompile ? "*" : ""}
                </Button>
                <ButtonGroup>
                    <ToggleButton
                        id="toggle-visual"
                        value={1}
                        checked={mode === "svg"}
                        active={mode === "svg"}
                        variant={
                            mode === "svg" ? "primary" : "outline-secondary"
                        }
                        onClick={() => setMode("svg")}
                        title="Render an SVG for visual display"
                    >
                        Visual
                    </ToggleButton>
                    <ToggleButton
                        id="toggle-tactile"
                        value={2}
                        checked={mode === "tactile"}
                        active={mode === "tactile"}
                        variant={
                            mode === "tactile" ? "primary" : "outline-secondary"
                        }
                        onClick={() => setMode("tactile")}
                        title="Render an SVG suitable for embossing or tactile display"
                    >
                        Tactile
                    </ToggleButton>
                </ButtonGroup>
                <div className="toolbar-actions">
                    <SplitButton
                        size="sm"
                        title={<><Download /> Download Graphic</>}
                        onClick={() => {
                            if (!compiledSource.startsWith("<svg")) {
                                throw new Error(
                                    "Cannot download non-SVG content: " +
                                    compiledSource,
                                );
                            }
                            const blob = new Blob([compiledSource], {
                                type: "image/svg+xml",
                            });
                            saveAs(blob, "figure.svg");
                        }}
                    >
                        <Dropdown.Item
                            onClick={() => {
                                if (!annotations.startsWith("<diagram")) {
                                    throw new Error(
                                        "Cannot download unknown annotations: " +
                                        annotations,
                                    );
                                }
                                const blob = new Blob([annotations], {
                                    type: "application/xml",
                                });
                                saveAs(blob, "figure.xml");
                            }}
                        >
                            <Download /> Download Annotations
                        </Dropdown.Item>
                        <Dropdown.Item
                            onClick={() => {
                                if (!annotations.startsWith("<diagram")) {
                                    throw new Error(
                                        "Cannot download unknown annotations: " +
                                        annotations,
                                    );
                                }
                                if (!compiledSource.startsWith("<svg")) {
                                    throw new Error(
                                        "Cannot download non-SVG content: " +
                                        compiledSource,
                                    );
                                }
                                const blob = new Blob([assembleCodeSnippet(compiledSource, annotations), scriptSnippet()], {
                                    type: "application/xml",
                                });
                                saveAs(blob, "figure.xml");
                            }}
                        >
                            <Download /> Download Code Snippet
                        </Dropdown.Item>
                    </SplitButton>
                </div>
            </Nav>
        </div>
    );
}

function AnnotateSvg({
    svg,
    annotations,
}: {
    svg: string;
    annotations: string;
}) {
    React.useEffect(() => {
        if (annotations) {
            diagcess.Base.init(true);
        }
    });

    return (
        <div className="ChemAccess-element">
            <div
                className="svg"
                dangerouslySetInnerHTML={{
                    __html: svg,
                }}
            ></div>
            <div
                className="cml"
                dangerouslySetInnerHTML={{
                    __html: annotations,
                }}
            ></div>
        </div>
    );
}

export const diagcessVersion = diagcess.version;
