import Editor from "@monaco-editor/react";
import { useStoreState, useStoreActions } from "../state";

/**
 * A source editor component for XML code using CodeMirror 6 directly.
 * This component allows the user to type PreFigure code.
 */
export function SourceEditor() {
    const source = useStoreState((state) => state.source);
    const setSource = useStoreActions((actions) => actions.setSource);

    return (
        <Editor
            height="100%"
            options={{ minimap: { enabled: false }, lineNumbers: "off" }}
            defaultLanguage="xml"
            value={source}
            onChange={(value) => value && setSource(value)}
        />
    );
}
