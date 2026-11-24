import { useEffect, useState } from "react";
import { Plus, UserRound } from "lucide-react";
import { actAs, openNewCritter, updateCritter } from "@/actions/critters";
import type { CritterSummary } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { SPECIES } from "@/components/critters/species";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { critterNameProblem } from "@/domain/critterId";
import { selectSelectedCritter, useCritterStore } from "@/stores/critterStore";
import { useUiStore } from "@/stores/uiStore";
import { CritterList } from "./CritterList";
import { ContextTab } from "./context/ContextTab";
import { FlowsTab } from "./flows/FlowsTab";
import { CritterSettingsTab } from "./settings/CritterSettingsTab";

/** Radix marks every open dialog and menu; Esc belongs to them then. */
const overlayOpen = (): boolean =>
  document.querySelector('[role="dialog"], [role="menu"], [role="listbox"][data-state="open"]') !== null;

/** Esc goes back to the request editor. */
function useEscapeToEditor() {
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || overlayOpen()) return;
      useUiStore.getState().setView("editor");
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}

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
        className="-mx-1 truncate rounded px-1 text-left text-lg font-semibold focus-ring hover:bg-accent/50"
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
        if (event.key === "Escape") {
          // Leave the view open: this Esc only cancels the rename.
          event.preventDefault();
          setDraft(null);
        }
      }}
      className="-mx-1 rounded border bg-background px-1 text-lg font-semibold outline-none focus:ring-1 focus:ring-ring aria-invalid:border-danger"
    />
  );
}

function CritterHeader({ critter }: { critter: CritterSummary }) {
  const active = useCritterStore((s) => s.activeId === critter.id);
  const species = SPECIES[critter.species];
  return (
    <header className="flex items-center gap-4 border-b p-5">
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
          <UserRound /> Stop acting as
        </Button>
      ) : (
        <Button onClick={() => void actAs(critter.id)}>Act as {critter.name}</Button>
      )}
    </header>
  );
}

function CritterDetail({ critter }: { critter: CritterSummary }) {
  return (
    <section className="flex min-w-0 flex-1 flex-col overflow-hidden">
      <CritterHeader critter={critter} />
      <Tabs defaultValue="context" className="flex min-h-0 flex-1 flex-col">
        <TabsList className="mx-5 mt-3">
          <TabsTrigger value="context">Context</TabsTrigger>
          <TabsTrigger value="flows">Flows</TabsTrigger>
          <TabsTrigger value="settings">Settings</TabsTrigger>
        </TabsList>
        <div className="min-h-0 flex-1 overflow-y-auto">
          <TabsContent value="context">
            <ContextTab critter={critter} />
          </TabsContent>
          {/* Full height: the flow editor scrolls its own panes. */}
          <TabsContent value="flows" className="h-full">
            <FlowsTab critter={critter} />
          </TabsContent>
          <TabsContent value="settings">
            <CritterSettingsTab key={critter.id} critter={critter} />
          </TabsContent>
        </div>
      </Tabs>
    </section>
  );
}

/** Shown until the project has a critter. */
function EmptyState() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
      <div className="flex gap-2" aria-hidden>
        {(["roadrunner", "tortoise", "elf-owl"] as const).map((species) => (
          <CritterSprite key={species} species={species} size={48} label={SPECIES[species].name} />
        ))}
      </div>
      <p className="max-w-sm text-sm text-muted-foreground">
        Send requests as different users. Each critter keeps its own variables and tokens.
      </p>
      <Button onClick={openNewCritter}>
        <Plus /> New critter
      </Button>
    </div>
  );
}

/** Replaces the editor while `view` is `"critters"` (Ctrl+B); Esc goes back. */
export function CrittersView() {
  useEscapeToEditor();
  const loaded = useCritterStore((s) => s.loaded);
  const count = useCritterStore((s) => s.critters.length);
  const selected = useCritterStore(selectSelectedCritter);
  if (!loaded) return null;
  if (count === 0) return <EmptyState />;
  return (
    <div className="flex h-full px-3 pb-3">
      <div className="flex min-h-0 flex-1 overflow-hidden rounded-xl border bg-card/70 shadow-lg shadow-shadow/20">
        <CritterList />
        {selected && <CritterDetail key={selected.id} critter={selected} />}
      </div>
    </div>
  );
}
