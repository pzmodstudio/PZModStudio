import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import "./i18n";
import { readUiMode } from "./utils/uiMode";

// Simple or Complete is decided before anything else writes a preference: a new install is told
// from one that was used before by what is already kept.
readUiMode();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
