import { useState } from "react";
import { createCritter, type CritterStart } from "@/actions/critters";
import type { Species } from "@/api/client";
import { SPECIES, SPECIES_IDS } from "@/components/critters/species";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { critterNameProblem, uniqueCritterId } from "@/domain/critterId";
import { cn } from "@/lib/utils";
import { useCritterStore } from "@/stores/critterStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { SpeciesGrid } from "./SpeciesGrid";

type StartKind = CritterStart["kind"];

/** The start chosen, or null while its env or critter is missing. */
function chosenStart(kind: StartKind, env: string, from: string): CritterStart | null {
  if (kind === "env") return env ? { kind, env } : null;
  if (kind === "duplicate") return from ? { kind, from } : null;
  return { kind };
}

/** Mounted per opening, so it starts fresh each time. */
function NewCritterForm({ onDone }: { onDone: () => void }) {
  const critters = useCritterStore((s) => s.critters);
  const envs = useProjectStore((s) => s.envs);
  const activeEnv = useProjectStore((s) => s.activeEnv);
  // Start with a species nobody has yet, so a second critter looks different.
  const unused = SPECIES_IDS.find((species) => !critters.some((c) => c.species === species));
  const [species, setSpecies] = useState<Species>(unused ?? "roadrunner");
  const [name, setName] = useState("");
  const [startKind, setStartKind] = useState<StartKind>("empty");
  const [env, setEnv] = useState(activeEnv ?? envs[0] ?? "");
  const [from, setFrom] = useState(critters[0]?.id ?? "");
  const [submitting, setSubmitting] = useState(false);

  const problem = critterNameProblem(name, critters);
  const id = uniqueCritterId(
    name,
    critters.map((c) => c.id),
  );
  const start = chosenStart(startKind, env, from);

  const submit = async () => {
    if (problem || !start || submitting) return;
    setSubmitting(true);
    const created = await createCritter(name.trim(), species, start);
    setSubmitting(false);
    if (created) onDone();
  };

  return (
    <form
      className="flex flex-col gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <DialogHeader>
        <DialogTitle>New bot</DialogTitle>
      </DialogHeader>

      <SpeciesGrid value={species} onChange={setSpecies} />
      <p className="-mt-2 text-center text-xs text-muted-foreground">{SPECIES[species].description}</p>

      <label className="flex flex-col gap-1.5 text-sm">
        <span className="font-medium">Name</span>
        <Input
          autoFocus
          value={name}
          placeholder="Bob the viewer"
          onChange={(event) => setName(event.target.value)}
          aria-invalid={!!name && !!problem}
        />
        <span className={cn("font-mono text-xs text-muted-foreground", name && problem && "text-danger")}>
          {name && problem ? problem : `critters/${id}/`}
        </span>
      </label>

      <fieldset className="flex flex-col gap-2 text-sm">
        <legend className="mb-1.5 font-medium">Start from</legend>
        <label className="flex items-center gap-2">
          <input type="radio" checked={startKind === "empty"} onChange={() => setStartKind("empty")} />
          Nothing
        </label>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-2">
            <input
              type="radio"
              disabled={envs.length === 0}
              checked={startKind === "env"}
              onChange={() => setStartKind("env")}
            />
            Variable names from
          </label>
          <Select value={env} onValueChange={setEnv} disabled={envs.length === 0}>
            <SelectTrigger className="h-7 w-40" onFocus={() => setStartKind("env")}>
              <SelectValue placeholder="No environments" />
            </SelectTrigger>
            <SelectContent>
              {envs.map((envName) => (
                <SelectItem key={envName} value={envName}>
                  {envName}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-2">
            <input
              type="radio"
              disabled={critters.length === 0}
              checked={startKind === "duplicate"}
              onChange={() => setStartKind("duplicate")}
            />
            A copy of
          </label>
          <Select value={from} onValueChange={setFrom} disabled={critters.length === 0}>
            <SelectTrigger className="h-7 w-40" onFocus={() => setStartKind("duplicate")}>
              <SelectValue placeholder="No bots yet" />
            </SelectTrigger>
            <SelectContent>
              {critters.map((critter) => (
                <SelectItem key={critter.id} value={critter.id}>
                  {critter.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </fieldset>

      <DialogFooter>
        <Button type="submit" disabled={!!problem || !start || submitting}>
          Create
        </Button>
      </DialogFooter>
    </form>
  );
}

export function NewCritterDialog() {
  const open = useUiStore((s) => s.newCritterOpen);
  const setOpen = useUiStore((s) => s.setNewCritterOpen);
  return (
    <Dialog open={open} onOpenChange={setOpen}>
      {/* No description: the title says it all. */}
      <DialogContent className="sm:max-w-lg" aria-describedby={undefined}>
        {open && <NewCritterForm onDone={() => setOpen(false)} />}
      </DialogContent>
    </Dialog>
  );
}
