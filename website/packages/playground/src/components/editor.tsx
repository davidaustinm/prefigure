import Editor from "@monaco-editor/react";
import { useEffect, useState } from "react";
import { useStoreState, useStoreActions } from "../state";
import { convert } from "@naman22khater/data-converter";

type Language = "xml" | "yaml";

function xmlToYaml(source: string): string {
  return convert(source, {from: 'xml', to: 'yaml'}).output;
}

function yamlToXml(source: string): string {
  return convert(source, {
    from: 'yaml',
    to: 'xml',
    serializeOptions: {
      declaration: false,
      rootElement: ''
    }
  }).output;
}

let init = 2;

export function SourceEditor() {
  const source = useStoreState((state) => state.source);
  const setSource = useStoreActions((actions) => actions.setSource);

  const [content, setContent] = useState<string>(source);
  const [language, setLanguage] = useState<Language>("xml");

  // Translate source when language changes
  useEffect(() => {
    try {
      if (!source.trim()) return;

      if (language === "yaml") {
        // XML → YAML
        setContent(xmlToYaml(content));
      } else {
        // YAML → XML
        if (init) {
          init--;
          return;
        }
        setContent(yamlToXml(content));
      }
    } catch {
      // Ignore conversion errors (e.g. invalid syntax while editing)
    }
  }, [language]);

  return (
    <div
      style={{
        height: "100%",
        width: "100%",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div style={{ flex: "0 0 auto" }}>
        <label>
          Language:&nbsp;
          <select
            value={language}
            onChange={(e) => setLanguage(e.target.value as Language)}
          >
            <option value="xml">XML</option>
            <option value="yaml">YAML</option>
          </select>
        </label>
      </div>

      <div style={{ flex: "1 1 auto", minHeight: 0 }}>
        <Editor
          width="100%"
          height="100%"
          language={language}
          value={content}
          options={{
            minimap: { enabled: false },
            lineNumbers: "off",
          }}
          onChange={(value) => {
            if (value !== undefined) {
              setSource(language === "xml" ? value : yamlToXml(value));
              setContent(value);
            }
          }}
        />
      </div>
    </div>
  );
}
