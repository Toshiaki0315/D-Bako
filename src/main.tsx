import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { SettingsView } from "./components/SettingsView";
import { viewFromSearch } from "./lib/view";
import "./App.css";

const view = viewFromSearch(window.location.search);
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {view === "settings" ? <SettingsView /> : <App />}
  </React.StrictMode>
);
