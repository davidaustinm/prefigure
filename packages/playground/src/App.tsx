import React from "react";
import { Container, Navbar, Nav, NavDropdown, Row, Col } from "react-bootstrap";
import { Check2 } from "react-bootstrap-icons";
import "bootstrap/dist/css/bootstrap.min.css";
import "./font.css";
import "./App.css";
import { SourceEditor } from "./components/editor";
import { diagcessVersion, Renderer } from "./components/renderer";
import { prefigBrowserApi } from "./worker/compat-api";
import { useStoreState, useStoreActions } from "./state";
import type { Engine } from "./state/model";

/** The engines offered in the "Versions" dropdown, in display order. */
const ENGINE_OPTIONS: { value: Engine; label: string; hint: string }[] = [
    {
        value: "pyodide",
        label: "Python",
        hint: "The Python package running in Pyodide",
    },
    {
        value: "wasm-mathjax",
        label: "Rust + MathJax",
        hint: "The Rust port (WebAssembly); math rendered by the browser's MathJax",
    },
    {
        value: "wasm-ratex",
        label: "Rust + RaTeX",
        hint: "The Rust port (WebAssembly); math rendered by the embedded pure-Rust RaTeX engine",
    },
];

function App() {
    const version = useStoreState((state) => state.prefigVersion);
    const engine = useStoreState((state) => state.engine);
    const setEngine = useStoreActions((actions) => actions.setEngine);

    return (
        <React.Fragment>
            <Navbar bg="primary" variant="dark">
                <Container>
                    <Navbar.Brand href="#">
                        <img src="./logo.svg" width={30} />
                        PreFigure Playground
                    </Navbar.Brand>
                    <Nav className="me-auto">
                        <Nav.Link
                            href="https://prefigure.org/docs/chap-examples.html"
                            target="_blank"
                        >
                            Examples
                        </Nav.Link>
                        <Nav.Link href="https://prefigure.org" target="_blank">
                            About
                        </Nav.Link>
                    </Nav>
                    <NavDropdown
                        className="version-menu bg-primary"
                        title="Versions"
                        align="end"
                    >
                        <NavDropdown.Header>Engine</NavDropdown.Header>
                        {ENGINE_OPTIONS.map(({ value, label, hint }) => (
                            <NavDropdown.Item
                                key={value}
                                as="button"
                                className="engine-option"
                                active={engine === value}
                                title={hint}
                                onClick={() => {
                                    if (engine !== value) {
                                        setEngine(value);
                                    }
                                }}
                            >
                                <span className="engine-check">
                                    {engine === value ? <Check2 /> : null}
                                </span>
                                {label}
                            </NavDropdown.Item>
                        ))}
                        <NavDropdown.Divider />
                        <div className="version-grid">
                            {(
                                [
                                    ["PreFigure", version],
                                    ["MathJax", prefigBrowserApi.mjVersion],
                                    ["SRE", prefigBrowserApi.sreVersion],
                                    ["diagcess", diagcessVersion],
                                ] as [string, string | undefined][]
                            ).map(([pkg, ver]) => (
                                <React.Fragment key={pkg}>
                                    <span className="version-label">
                                        {pkg}:
                                    </span>
                                    <code className="version-value">
                                        {ver || "Unknown"}
                                    </code>
                                </React.Fragment>
                            ))}
                        </div>
                    </NavDropdown>
                </Container>
            </Navbar>
            <Container fluid className="full-container">
                <Row className="full-container">
                    <Col className="editor-panel panel">
                        <h2>Source Code</h2>
                        <div className="editor-container">
                            <SourceEditor />
                        </div>
                    </Col>
                    <Col className="bg-light panel">
                        <h2>Rendered Content</h2>
                        <Renderer />
                    </Col>
                </Row>
            </Container>
        </React.Fragment>
    );
}

export default App;
