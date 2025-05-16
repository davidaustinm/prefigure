import React from "react";
import { Container, Navbar, Nav, Row, Col } from "react-bootstrap";
import "bootstrap/dist/css/bootstrap.min.css";
import "./font.css";
import "./App.css";
import { SourceEditor } from "./components/editor";
import { Renderer } from "./components/renderer";
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
                        <Nav.Link href="https://github.com/davidaustinm/prefigure/tree/main/prefig/resources/examples" target="_blank">Examples</Nav.Link>
                        <Nav.Link href="https://prefigure.org" target="_blank">About</Nav.Link>
                    </Nav>
                    <Nav.Item className="text-light small">
                        PreFigure Version:{" "}
                        {version ? <code>{version}</code> : "Unknown"}
                    </Nav.Item>
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
