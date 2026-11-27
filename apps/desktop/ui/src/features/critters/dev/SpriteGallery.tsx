import { useState } from "react";
import { CritterSprite, type SpriteSize } from "@/components/critters/CritterSprite";
import { SPECIES, SPECIES_IDS, type CritterState } from "@/components/critters/species";
import { cn } from "@/lib/utils";
import { THEMES, type ThemeEntry } from "@/theme/themes";

/**
 * Dev only (`npm run dev`, then open `/#sprites`): every species in every
 * state, size and theme, for reviewing the art. Not part of the app.
 */

const STATES: readonly CritterState[] = ["idle", "work", "pass", "fail"];
const SIZES: readonly SpriteSize[] = [16, 32, 48];
const SHOWCASE = new Set(["cactus", "desert-day", "phosphor", "win95"]);

function ThemePanel({ theme, replay }: { theme: ThemeEntry; replay: number }) {
  return (
    <section
      data-theme={theme.id}
      className={cn(
        "flex flex-col gap-3 rounded-xl border bg-background p-4 text-foreground",
        theme.scheme === "dark" && "dark",
      )}
    >
      <h2 className="text-sm font-semibold">
        {theme.name} <span className="font-normal text-muted-foreground">({theme.scheme})</span>
      </h2>
      <table className="border-separate border-spacing-x-4 border-spacing-y-2 text-xs">
        <thead className="text-left text-muted-foreground">
          <tr>
            <th className="font-normal">Species</th>
            {SIZES.map((size) => (
              <th key={size} className="font-normal">
                idle {size}
              </th>
            ))}
            {STATES.slice(1).map((state) => (
              <th key={state} className="font-normal">
                {state} 48
              </th>
            ))}
            <th className="font-normal">asleep</th>
          </tr>
        </thead>
        <tbody>
          {SPECIES_IDS.map((species) => {
            const { name } = SPECIES[species];
            return (
              <tr key={species}>
                <td className="pr-2 whitespace-nowrap">{name}</td>
                {SIZES.map((size) => (
                  <td key={size}>
                    <CritterSprite species={species} size={size} label={`${name}, idle`} />
                  </td>
                ))}
                {STATES.slice(1).map((state) => (
                  <td key={`${state}:${replay}`}>
                    <CritterSprite species={species} state={state} size={48} label={`${name}, ${state}`} />
                  </td>
                ))}
                <td>
                  <CritterSprite species={species} size={32} label={`${name}, asleep`} asleep />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </section>
  );
}

export function SpriteGallery() {
  const [shown, setShown] = useState<"showcase" | "all">("showcase");
  const [replay, setReplay] = useState(0);
  const themes = THEMES.filter((theme) => shown === "all" || SHOWCASE.has(theme.id));
  return (
    <main className="flex h-screen flex-col gap-4 overflow-auto bg-background p-6 text-foreground">
      <header className="flex flex-wrap items-center gap-3">
        <h1 className="text-lg font-semibold">Bot sprites</h1>
        {SPECIES_IDS.map((species) => (
          <span key={species} className="text-xs text-muted-foreground">
            <b className="text-foreground">{SPECIES[species].name}</b>: {SPECIES[species].description}
          </span>
        ))}
        <div className="ml-auto flex gap-2">
          <button type="button" className="rounded border px-2 py-1 text-sm" onClick={() => setReplay((n) => n + 1)}>
            Replay pass/fail
          </button>
          <button
            type="button"
            className="rounded border px-2 py-1 text-sm"
            onClick={() => setShown(shown === "all" ? "showcase" : "all")}
          >
            {shown === "all" ? "Show 4 themes" : `Show all ${THEMES.length} themes`}
          </button>
        </div>
      </header>
      <div className="grid gap-4 xl:grid-cols-2">
        {themes.map((theme) => (
          <ThemePanel key={theme.id} theme={theme} replay={replay} />
        ))}
      </div>
    </main>
  );
}
