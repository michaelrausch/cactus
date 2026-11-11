import { useEffect, useState } from "react";
import { KeyRound } from "lucide-react";
import { critterOverrides, saveCritterSecrets } from "@/actions/critters";
import type { CritterSummary, EnvFile } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EnvLayerEditor } from "@/features/env/EnvLayerEditor";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/stores/projectStore";
import { CapturedList } from "./CapturedList";

/** One input per `secret: true` variable with no value anywhere yet. */
function MissingSecrets({ critter }: { critter: CritterSummary }) {
  const [values, setValues] = useState<Record<string, string>>({});
  const filled = Object.fromEntries(Object.entries(values).filter(([, value]) => value !== ""));
  if (critter.missingSecrets.length === 0) return null;
  return (
    <section className="flex flex-col gap-3 rounded-lg border border-warning/40 bg-warning/10 p-4">
      <h3 className="flex items-center gap-2 text-sm font-semibold">
        <KeyRound className="size-4 text-warning" /> Missing secrets
      </h3>
      <p className="text-xs text-muted-foreground">
        {critter.name} can't send requests that use these until they have values. They're kept in
        <span className="font-mono"> .cactus/critters/{critter.id}/secrets.json</span>, never committed.
      </p>
      <form
        className="flex flex-col gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          void saveCritterSecrets(critter.id, filled).then((saved) => saved && setValues({}));
        }}
      >
        {critter.missingSecrets.map((name) => (
          <label key={name} className="grid grid-cols-[minmax(8rem,auto)_1fr] items-center gap-3 text-sm">
            <span className="font-mono">{name}</span>
            <Input
              type="password"
              autoComplete="off"
              value={values[name] ?? ""}
              onChange={(event) => setValues({ ...values, [name]: event.target.value })}
            />
          </label>
        ))}
        <Button type="submit" size="sm" className="self-end" disabled={Object.keys(filled).length === 0}>
          Save secrets
        </Button>
      </form>
    </section>
  );
}

type Layer = "critter" | "critterSecrets";

const LAYER_LABELS: Record<Layer, string> = { critter: "Variables", critterSecrets: "Secrets" };

/** "Overrides BASE_URL (staging) …". Mount with a `key` that changes when it should look again. */
function Overrides({ critterId }: { critterId: string }) {
  const [overrides, setOverrides] = useState<[string, string][]>([]);
  useEffect(() => {
    let current = true;
    void critterOverrides(critterId).then((found) => current && setOverrides(found));
    return () => {
      current = false;
    };
  }, [critterId]);
  if (overrides.length === 0) return null;
  return (
    <p className="flex flex-wrap gap-1.5 text-xs text-muted-foreground">
      Overrides
      {overrides.map(([name, label]) => (
        <span key={name} className="rounded bg-muted px-1.5 font-mono">
          {name} <span className="font-sans">({label})</span>
        </span>
      ))}
    </p>
  );
}

/** The critter's variables and secrets, each edited like an env layer, and what they override. */
function Variables({ critter }: { critter: CritterSummary }) {
  const project = useProjectStore((s) => s.project);
  const activeEnv = useProjectStore((s) => s.activeEnv);
  const [layer, setLayer] = useState<Layer>("critter");
  /** Bumped on save, so the overrides are looked up again. */
  const [version, setVersion] = useState(0);

  if (!project) return null;
  const file: EnvFile = { kind: layer, name: critter.id };
  return (
    <section className="flex flex-col gap-3">
      <div role="tablist" aria-label="Layer" className="flex gap-1">
        {(Object.keys(LAYER_LABELS) as Layer[]).map((option) => (
          <button
            key={option}
            type="button"
            role="tab"
            aria-selected={option === layer}
            onClick={() => setLayer(option)}
            className={cn(
              "rounded-md px-2.5 py-1 text-sm text-muted-foreground focus-ring hover:text-foreground",
              option === layer && "bg-accent text-foreground",
            )}
          >
            {LAYER_LABELS[option]}
          </button>
        ))}
      </div>
      <EnvLayerEditor
        key={`${critter.id}:${layer}`}
        inline
        project={project}
        file={file}
        onSaved={() => setVersion((n) => n + 1)}
      />
      <Overrides key={`${critter.id}:${activeEnv}:${version}`} critterId={critter.id} />
    </section>
  );
}

/** Missing secrets, the critter's variables and secrets, and what its requests captured. */
export function ContextTab({ critter }: { critter: CritterSummary }) {
  return (
    <div className="flex flex-col gap-8 p-5">
      <MissingSecrets key={critter.id} critter={critter} />
      <Variables critter={critter} />
      <CapturedList critter={critter} />
    </div>
  );
}
