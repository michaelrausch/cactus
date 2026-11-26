import type { ToolDef } from "@/features/critters/flows/tools/types";
import type { Theme } from "@/theme/themes";

/** The tools only the Pro edition has (`core/src/tools/mod.rs`). */
export type ProToolId = "webhook" | "slack" | "fuzz" | "headers" | "cert" | "ratelimit" | "loadtest";

/** What each edition's module (`@edition`) exports. */
export interface Edition {
  PRO: boolean;
  PRO_TOOLS: { readonly [T in ProToolId]?: ToolDef<T> };
}

/** What each edition's `@edition/themes` exports: kept apart so `theme/themes.ts` needn't load the tools. */
export interface EditionThemes {
  /** Listed after the free edition's themes. */
  PRO_THEMES: readonly Theme[];
}
