import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./app/App";
import { useSettingsStore } from "./stores/settingsStore";
import { applyTheme, resolveTheme } from "./theme/themes";
import "./index.css";

const root = document.getElementById("root");
if (!root) throw new Error("index.html is missing the #root element");

/** critters.css stops idle critter motion while this is set. */
const applyCritterMotion = (animate: boolean) =>
  document.documentElement.toggleAttribute("data-critters-still", !animate);

// Before the first render, so the saved theme never flashes the default.
applyTheme(resolveTheme(useSettingsStore.getState().themeId));
applyCritterMotion(useSettingsStore.getState().animateCritters);
useSettingsStore.subscribe((state, previous) => {
  if (state.themeId !== previous.themeId) applyTheme(resolveTheme(state.themeId));
  if (state.animateCritters !== previous.animateCritters) applyCritterMotion(state.animateCritters);
});

// No browser context menu (and its Inspect Element) in the shipped app; kept in dev for debugging.
if (import.meta.env.PROD) {
  document.addEventListener("contextmenu", (event) => event.preventDefault());
}

// Dev only: `/#sprites` shows every critter sprite for review. The import is
// dropped from production builds along with the branch.
if (import.meta.env.DEV && window.location.hash === "#sprites") {
  void import("./features/critters/dev/SpriteGallery").then(({ SpriteGallery }) =>
    createRoot(root).render(
      <StrictMode>
        <SpriteGallery />
      </StrictMode>,
    ),
  );
} else {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
