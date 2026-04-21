import React from "react";
import { Container, Navbar, Nav, NavDropdown, Row, Col } from "react-bootstrap";
import "bootstrap/dist/css/bootstrap.min.css";
import "./font.css";
import "./App.css";
import { SourceEditor } from "./components/editor";
import { diagcessVersion, Renderer } from "./components/renderer";
import { prefigBrowserApi } from "./worker/compat-api";
import { useStoreState } from "./state";

function App() {
    const version = useStoreState((state) => state.prefigVersion);

    return (
        <React.Fragment>
            <Navbar bg="primary" variant="dark">
                <Container>
                    <Navbar.Brand href="#">
                        <img src="./logo.svg" width={30} />
                        PreFigure Playground
                    </Navbar.Brand>
                    <Nav className="me-auto">
                        <Nav.Link href="https://prefigure.org/docs/chap-examples.html" target="_blank">Examples</Nav.Link>
                        <Nav.Link href="https://prefigure.org" target="_blank">About</Nav.Link>
                    </Nav>
                    <NavDropdown className="version-menu bg-primary" title="Versions" align="end">
                        <div className="version-grid">
                            {([
                                ["PreFigure", version],
                                ["MathJax", prefigBrowserApi.mjVersion],
                                ["SRE", prefigBrowserApi.sreVersion],
                                ["diagcess", diagcessVersion],
                            ] as [string, string | undefined][]).map(([pkg, ver]) => (
                                <React.Fragment key={pkg}>
                                    <span className="version-label">{pkg}:</span>
                                    <code className="version-value">{ver || "Unknown"}</code>
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
