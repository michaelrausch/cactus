import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./app/App";
import { useSettingsStore } from "./stores/settingsStore";
import { applyTheme, resolveTheme } from "./theme/themes";
import "./index.css";

const root = document.getElementById("root");
if (!root) throw new Error("index.html is missing the #root element");

// Before the first render, so the saved theme never flashes the default.
applyTheme(resolveTheme(useSettingsStore.getState().themeId));
useSettingsStore.subscribe((state, previous) => {
  if (state.themeId !== previous.themeId) applyTheme(resolveTheme(state.themeId));
});

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
