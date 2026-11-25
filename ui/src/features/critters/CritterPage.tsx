import { useState } from "react";
import { UserRound } from "lucide-react";
import { actAs, updateCritter } from "@/actions/critters";
import type { CritterSummary } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { SPECIES } from "@/components/critters/species";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { critterNameProblem } from "@/domain/critterId";
import { useCritterStore } from "@/stores/critterStore";
import { useUiStore, type CritterTab } from "@/stores/uiStore";
import { ContextTab } from "./context/ContextTab";
import { CritterSettingsTab } from "./settings/CritterSettingsTab";

/** The name, renamed in place: click, type, Enter (Esc cancels). */
function EditableName({ critter }: { critter: CritterSummary }) {
  const critters = useCritterStore((s) => s.critters);
  const [draft, setDraft] = useState<string | null>(null);
  if (draft === null) {
    return (
      <button
        type="button"
        title="Rename"
        onClick={() => setDraft(critter.name)}
        className="-mx-1 truncate rounded px-1 text-left text-lg font-semibold tracking-tight focus-ring hover:bg-accent/50"
      >
        {critter.name}
      </button>
    );
  }
  const problem = critterNameProblem(
    draft,
    critters.filter((c) => c.id !== critter.id),
  );
  const commit = () => {
    const name = draft.trim();
    setDraft(null);
    if (!problem && name !== critter.name) void updateCritter(critter.id, (saved) => ({ ...saved, name }));
  };
  return (
    <input
      autoFocus
      aria-label="Name"
      aria-invalid={!!problem}
      title={problem ?? undefined}
      value={draft}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
        if (event.key === "Escape") setDraft(null);
      }}
      className="-mx-1 rounded border bg-background px-1 text-lg font-semibold tracking-tight outline-none focus:ring-1 focus:ring-ring aria-invalid:border-danger"
    />
  );
}

function CritterHeader({ critter }: { critter: CritterSummary }) {
  const active = useCritterStore((s) => s.activeId === critter.id);
  const species = SPECIES[critter.species];
  return (
    <header className="flex items-center gap-4 border-b px-5 py-4">
      <CritterSprite species={critter.species} size={48} label={`${critter.name}, ${species.name}`} seed={critter.id} />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <EditableName key={critter.id} critter={critter} />
        <p className="truncate text-sm text-muted-foreground">
          {species.name}
          {critter.notes && ` · ${critter.notes}`}
        </p>
        {critter.error && <p className="truncate text-xs text-danger">{critter.error}</p>}
      </div>
      {active ? (
        <Button variant="outline" onClick={() => void actAs(null)}>
          <UserRound /> Stop sending as
        </Button>
      ) : (
        <Button onClick={() => void actAs(critter.id)}>Send as {critter.name}</Button>
      )}
    </header>
  );
}

/** A critter in the main pane: who it is, its captured context and its settings. Its flows are in the sidebar. */
export function CritterPage({ critter }: { critter: CritterSummary }) {
  const tab = useUiStore((s) => s.critterTab);
  const setTab = useUiStore((s) => s.setCritterTab);
  return (
    <section className="flex h-full min-w-0 flex-col overflow-hidden">
      <CritterHeader critter={critter} />
      <Tabs value={tab} onValueChange={(value) => setTab(value as CritterTab)} className="flex min-h-0 flex-1 flex-col">
        <TabsList className="mx-5 mt-3">
          <TabsTrigger value="context">Context</TabsTrigger>
          <TabsTrigger value="settings">Settings</TabsTrigger>
        </TabsList>
        <div className="min-h-0 flex-1 overflow-y-auto">
          <TabsContent value="context">
            <ContextTab critter={critter} />
          </TabsContent>
          <TabsContent value="settings">
            <CritterSettingsTab key={critter.id} critter={critter} />
          </TabsContent>
        </div>
      </Tabs>
    </section>
  );
}
