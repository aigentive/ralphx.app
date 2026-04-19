import * as React from "react";
import * as ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { syncThemeAttributesFromStore } from "./stores/themeStore";
import "./styles/globals.css";

// Reassert persisted theme / motion / font-scale on DOM after main.tsx loads.
// The inline script in index.html handles the pre-hydration case; this keeps
// attributes in sync with the Zustand store for any in-flight settings
// changes that happened between the two loads.
syncThemeAttributesFromStore();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
