import React from "react";
import { useStoreActions, useStoreState } from "../state";
import {
    Button,
    ButtonGroup,
    Nav,
    Spinner,
    ToggleButton,
    ToggleButtonGroup,
} from "react-bootstrap";
import { Download } from "react-bootstrap-icons";

/**
 * Renders and displays the currently active PreFigure source code.
 */
export function Renderer() {
    const loadPyodide = useStoreActions((actions) => actions.loadPyodide);
    const status = useStoreState((state) => state.status);
    const compiledSource = useStoreState((state) => state.compiledSource);
    const compile = useStoreActions((actions) => actions.compile);

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
            <div className="render-content">{compiledSource}</div>
            <Nav>
                <Button variant="success" onClick={() => compile()}>
                    Compile
                </Button>
                Render mode:{" "}
                <ButtonGroup>
                    <ToggleButton id="toggle-visual" value={1} checked={true}>
                        Visual
                    </ToggleButton>
                    <ToggleButton id="toggle-visual" value={2} checked={false}>
                        Tactile
                    </ToggleButton>
                </ButtonGroup>
                <Button size="sm">
                    <Download /> Download
                </Button>
            </Nav>
        </div>
    );
}
