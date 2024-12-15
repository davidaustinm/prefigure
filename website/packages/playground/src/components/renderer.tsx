import React from "react";
import { useStoreActions, useStoreState } from "../state";
import {
    Button,
    ButtonGroup,
    Nav,
    Spinner,
    ToggleButton,
} from "react-bootstrap";
import { Download } from "react-bootstrap-icons";
import { saveAs } from "file-saver";

/**
 * Renders and displays the currently active PreFigure source code.
 */
export function Renderer() {
    const loadPyodide = useStoreActions((actions) => actions.loadPyodide);
    const status = useStoreState((state) => state.status);
    const compiledSource = useStoreState((state) => state.compiledSource);
    const compile = useStoreActions((actions) => actions.compile);
    const mode = useStoreState((state) => state.compileMode);
    const setMode = useStoreActions((actions) => actions.setCompileMode);
    const needsCompile = useStoreState((state) => state.needsCompile);

    // We add `viewBox="..."` and `preserveAspectRatio="xMidYMid meet"` to the SVG to make sure it scales correctly
    // in our display region.
    const sourceForDisplay = React.useMemo(() => {
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

    if (status === "loadingPyodide") {
        return (
            <div className="loading">
                <Spinner animation="border" style={{ marginRight: "5px" }} />{" "}
                Loading Pyodide...
            </div>
        );
    }

    return (
        <div className="render-frame">
            <div className="render-content">
                {sourceForDisplay.startsWith("<svg") ? (
                    <div
                        className="rendered-svg"
                        dangerouslySetInnerHTML={{ __html: sourceForDisplay }}
                    ></div>
                ) : (
                    compiledSource
                )}
            </div>
            <Nav className="render-toolbar">
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
                        id="toggle-visual"
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
                <Button
                    size="sm"
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
                    <Download /> Download
                </Button>
            </Nav>
        </div>
    );
}
