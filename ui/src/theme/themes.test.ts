import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { DEFAULT_THEME, THEMES, applyTheme, resolveTheme } from "./themes";

/** Each `[data-theme…] { … }` block in themes.css: its id (null for the base) and the tokens it sets. */
function themeBlocks(source: string): { id: string | null; tokens: string[] }[] {
  const withoutComments = source.replace(/\/\*[\s\S]*?\*\//g, "");
  return [...withoutComments.matchAll(/\[data-theme(?:="([^"]+)")?\]\s*\{([^}]*)\}/g)].map((match) => ({
    id: match[1] ?? null,
    tokens: [...(match[2] ?? "").matchAll(/(--[\w-]+)\s*:/g)].map((token) => token[1] ?? ""),
  }));
}

// Read from disk: Vitest replaces CSS imports, `?raw` included, with empty strings.
const css = readFileSync(path.join(import.meta.dirname, "themes.css"), "utf8");
const blocks = themeBlocks(css);
const base = blocks.find((block) => block.id === null);
const named = blocks.filter((block) => block.id !== null);

describe("themes.css", () => {
  it("has one base block that sets the tokens components rely on", () => {
    expect(blocks.filter((block) => block.id === null)).toHaveLength(1);
    expect(base?.tokens).toEqual(
      expect.arrayContaining([
        "--background",
        "--foreground",
        "--brand-1",
        "--brand-2",
        "--brand-3",
        "--brand-foreground",
        "--flower",
        "--glow-1",
        "--glow-2",
        "--glow-intensity",
        "--app-font",
        "--app-backdrop",
        "--app-overlay",
        "--success",
        "--warning",
        "--danger",
        "--info",
        "--method-get",
        "--method-post",
        "--method-put",
        "--method-patch",
        "--method-delete",
        "--syntax-key",
        "--syntax-string",
        "--syntax-number",
        "--syntax-literal",
        "--template-resolved",
        "--captured",
        "--shadow",
      ]),
    );
  });

  it("has a block for every listed theme except the default, and no others", () => {
    const listed = THEMES.map((theme) => theme.id).filter((id) => id !== DEFAULT_THEME);
    expect(named.map((block) => block.id).toSorted()).toEqual(listed.toSorted());
  });

  it("only overrides tokens the base defines, so a typo can't slip through", () => {
    const known = new Set(base?.tokens);
    for (const block of named) {
      expect({ id: block.id, unknown: block.tokens.filter((token) => !known.has(token)) }).toEqual({
        id: block.id,
        unknown: [],
      });
    }
  });

  it("names only listed themes anywhere, Extras included", () => {
    const ids = new Set<string>(THEMES.map((theme) => theme.id));
    const mentioned = [...css.matchAll(/\[data-theme="([^"]+)"\]/g)].map((match) => match[1] ?? "");
    expect(mentioned.filter((id) => !ids.has(id))).toEqual([]);
  });

  it("sets a colour scheme in each light theme, since the base is dark", () => {
    for (const theme of THEMES.filter((t) => t.scheme === "light")) {
      const block = css.slice(css.indexOf(`[data-theme="${theme.id}"]`));
      expect(block.slice(0, block.indexOf("}"))).toContain("color-scheme: light");
    }
  });
});

/** App components (not the generated shadcn ones in `components/ui`), relative to `src`. */
const SRC = path.join(import.meta.dirname, "..");
const appComponents = (readdirSync(SRC, { recursive: true }) as string[]).filter(
  (file) => file.endsWith(".tsx") && !file.startsWith(path.join("components", "ui")),
);
const PALETTE_CLASS =
  /\b(?:text|bg|border|ring|shadow|fill|stroke|from|to|via|decoration)-(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}\b|\b(?:text|bg|shadow)-(?:black|white)\b/;

describe("components", () => {
  it("take colours from theme tokens, never Tailwind's fixed palette", () => {
    const offenders = appComponents.filter((file) => PALETTE_CLASS.test(readFileSync(path.join(SRC, file), "utf8")));
    expect(offenders).toEqual([]);
  });
});

describe("resolveTheme", () => {
  it("falls back to the default for unknown or missing ids", () => {
    expect(resolveTheme("phosphor").id).toBe("phosphor");
    expect(resolveTheme("removed-theme").id).toBe(DEFAULT_THEME);
    expect(resolveTheme(undefined).id).toBe(DEFAULT_THEME);
  });
});

describe("applyTheme", () => {
  it("sets data-theme and toggles .dark from the scheme", () => {
    const classes = new Set<string>(["dark"]);
    const root = {
      dataset: {} as Record<string, string>,
      classList: { toggle: (name: string, on: boolean) => (on ? classes.add(name) : classes.delete(name)) },
    } as unknown as HTMLElement;

    applyTheme(resolveTheme("desert-day"), root);
    expect(root.dataset.theme).toBe("desert-day");
    expect(classes.has("dark")).toBe(false);

    applyTheme(resolveTheme("synthwave"), root);
    expect(classes.has("dark")).toBe(true);
  });
});
