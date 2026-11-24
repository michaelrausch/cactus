import { useRef, type KeyboardEvent } from "react";
import type { Species } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { SPECIES, SPECIES_IDS } from "@/components/critters/species";
import { cn } from "@/lib/utils";

/** The species grid's width, for moving up and down with the arrow keys. */
const GRID_COLUMNS = 4;

const ARROW_STEPS: Record<string, number> = {
  ArrowLeft: -1,
  ArrowRight: 1,
  ArrowUp: -GRID_COLUMNS,
  ArrowDown: GRID_COLUMNS,
};

/** A 4×2 radio grid of species, moved through with the arrow keys. */
export function SpeciesGrid({ value, onChange }: { value: Species; onChange: (species: Species) => void }) {
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const onKeyDown = (event: KeyboardEvent) => {
    const step = ARROW_STEPS[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const count = SPECIES_IDS.length;
    const next = (SPECIES_IDS.indexOf(value) + step + count) % count;
    const species = SPECIES_IDS[next];
    if (!species) return;
    onChange(species);
    buttons.current[next]?.focus();
  };
  return (
    <div role="radiogroup" aria-label="Species" className="grid grid-cols-4 gap-1.5" onKeyDown={onKeyDown}>
      {SPECIES_IDS.map((species, index) => {
        const info = SPECIES[species];
        const selected = species === value;
        return (
          <button
            key={species}
            ref={(button) => {
              buttons.current[index] = button;
            }}
            type="button"
            role="radio"
            aria-checked={selected}
            tabIndex={selected ? 0 : -1}
            title={info.description}
            onClick={() => onChange(species)}
            className={cn(
              "flex flex-col items-center gap-1 rounded-lg p-2 ring-1 ring-transparent transition outline-none hover:bg-accent/50 focus-visible:ring-ring",
              selected && "bg-accent/60 ring-brand-2/70",
            )}
          >
            <CritterSprite species={species} size={32} label={info.name} />
            <span className="text-xs font-medium">{info.name}</span>
          </button>
        );
      })}
    </div>
  );
}
