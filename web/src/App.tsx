import { ReactFlowProvider } from "@xyflow/react";
import { useState } from "react";

import { Diagram } from "./Diagram";
import { useSchema } from "./hooks/useSchema";
import { readViewState, type ViewState } from "./viewState";

import "@xyflow/react/dist/style.css";
// Tokens first: theme.css reads them, and they are generated from the Rust
// definition so the layout and the browser size a node identically.
import "./generated/tokens.css";
import "./theme.css";

/**
 * The shell: load `schema.json`, then hand it to the canvas.
 *
 * The three states a reader can land in — loading, unreadable, drawn — are all
 * here, and the `ReactFlowProvider` is the reason the canvas is a component of
 * its own: `useReactFlow` and `useViewport` only work inside it.
 */
export function App() {
  const { schema, error } = useSchema();
  // Read once, on the first render: the URL goes on to follow the view, so
  // re-reading it later would mean reacting to CrabER's own writes.
  const [initial] = useState<ViewState>(() => readViewState());

  if (error) {
    return (
      <div className="craber-message">
        <p>{error}</p>
        <p>
          Generate one with{" "}
          <code>cargo run -p craber -- export --file fixtures/tiny.sql -o web/public/schema.json</code>
        </p>
      </div>
    );
  }
  if (!schema) {
    return <div className="craber-message">Loading…</div>;
  }

  return (
    <ReactFlowProvider>
      <Diagram schema={schema} initial={initial} />
    </ReactFlowProvider>
  );
}
