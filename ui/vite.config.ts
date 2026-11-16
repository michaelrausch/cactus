import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

/** Output of a git command, or `fallback` outside a checkout or without git. */
function git(args: string, fallback: string): string {
  try {
    return execSync(`git ${args}`, { cwd: import.meta.dirname, stdio: ["ignore", "pipe", "ignore"] })
      .toString()
      .trim();
  } catch {
    return fallback;
  }
}

const tauriConfig = JSON.parse(
  readFileSync(path.resolve(import.meta.dirname, "../src-tauri/tauri.conf.json"), "utf8"),
) as { version: string };

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "./src") } },
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "es2022", chunkSizeWarningLimit: 1000 },
  // Shown in Settings → About.
  define: {
    "import.meta.env.APP_VERSION": JSON.stringify(tauriConfig.version),
    "import.meta.env.GIT_SHA": JSON.stringify(git("rev-parse --short HEAD", "unknown")),
    "import.meta.env.GIT_DIRTY": JSON.stringify(git("status --porcelain", "") !== ""),
    "import.meta.env.BUILD_DATE": JSON.stringify(new Date().toISOString()),
  },
});
