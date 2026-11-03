import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

// Stylesheets before `App`, whose components import their own: CSS lands in
// import order, so React Flow's defaults come before the overrides in
// Diagram.css, and the tokens before everything that reads them — they are
// generated from the Rust definition so the layout and the browser size a node
// identically.
import "@xyflow/react/dist/style.css";
import "@/generated/tokens.css";
import "./theme.css";

import { App } from "./App";

const container = document.getElementById("root");
if (!container) throw new Error("missing #root");

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
