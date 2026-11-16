import { useState } from "react";
import { Play, UserRound } from "lucide-react";
import { actAs } from "@/actions/critters";
import { runFlow } from "@/actions/flows";
import { selectScenario } from "@/actions/scenario";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { MethodBadge } from "@/components/MethodBadge";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { useCritterStore } from "@/stores/critterStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

/** Typed first, these switch the palette from requests to critters or flows. */
const CRITTER_PREFIX = "@";
const FLOW_PREFIX = ">";

type Mode = "requests" | "critters" | "flows";

const modeOf = (search: string): Mode => {
  if (search.startsWith(CRITTER_PREFIX)) return "critters";
  if (search.startsWith(FLOW_PREFIX)) return "flows";
  return "requests";
};

const EMPTY_TEXT: Record<Mode, string> = {
  requests: "No match",
  critters: "No critter matches",
  flows: "No flow matches",
};

function RequestItems({ close }: { close: () => void }) {
  const entries = useProjectStore((s) => s.entries);
  return entries.map((entry) => (
    <CommandItem
      key={entry.relPath}
      value={`${entry.title} ${entry.relPath}`}
      onSelect={() => {
        close();
        void selectScenario(entry.relPath);
      }}
    >
      <MethodBadge method={entry.method} className="w-12 text-2xs" />
      <span className="flex-1 truncate">{entry.title}</span>
      <span className="truncate text-xs text-muted-foreground">{entry.relPath}</span>
    </CommandItem>
  ));
}

/** `@bob` acts as Bob; `@none` stops. */
function CritterItems({ close }: { close: () => void }) {
  const critters = useCritterStore((s) => s.critters);
  const activeId = useCritterStore((s) => s.activeId);
  const choose = (id: string | null) => {
    close();
    void actAs(id);
  };
  return (
    <CommandGroup heading="Send as">
      <CommandItem value={`${CRITTER_PREFIX}none no critter`} onSelect={() => choose(null)}>
        <UserRound className="text-muted-foreground" />
        <span className="flex-1">No critter</span>
        {!activeId && <span className="text-xs text-muted-foreground">active</span>}
      </CommandItem>
      {critters.map((critter) => (
        <CommandItem
          key={critter.id}
          value={`${CRITTER_PREFIX}${critter.id} ${critter.name}`}
          onSelect={() => choose(critter.id)}
        >
          <CritterSprite species={critter.species} size={16} label={critter.name} seed={critter.id} />
          <span className="flex-1 truncate">{critter.name}</span>
          {critter.id === activeId && <span className="text-xs text-muted-foreground">active</span>}
        </CommandItem>
      ))}
    </CommandGroup>
  );
}

/** `>health` runs a flow as its critter. */
function FlowItems({ close }: { close: () => void }) {
  const critters = useCritterStore((s) => s.critters);
  return critters
    .filter((critter) => critter.flows.length > 0)
    .map((critter) => (
      <CommandGroup key={critter.id} heading={critter.name}>
        {critter.flows.map((flow) => (
          <CommandItem
            key={flow.id}
            value={`${FLOW_PREFIX}${critter.id} ${flow.id} ${critter.name} ${flow.name}`}
            disabled={!!flow.error}
            onSelect={() => {
              close();
              void runFlow(critter.id, flow.id);
            }}
          >
            <Play className="text-muted-foreground" />
            <span className="flex-1 truncate">{flow.name}</span>
            <span className="truncate text-xs text-muted-foreground">
              {flow.error ? "doesn't parse" : `${flow.steps} steps`}
            </span>
          </CommandItem>
        ))}
      </CommandGroup>
    ));
}

/**
 * Command palette (Ctrl+P): jump to any request by title or path; type `@`
 * to act as a critter, or `>` to run a flow.
 */
export function ScenarioSwitcher() {
  const open = useUiStore((s) => s.switcherOpen);
  const setOpen = useUiStore((s) => s.setSwitcherOpen);
  const [search, setSearch] = useState("");
  const mode = modeOf(search);
  const close = () => setOpen(false);

  return (
    <CommandDialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) setSearch("");
      }}
    >
      <CommandInput
        value={search}
        onValueChange={setSearch}
        placeholder={`Jump to request… (${CRITTER_PREFIX} critters, ${FLOW_PREFIX} flows)`}
      />
      <CommandList>
        <CommandEmpty>{EMPTY_TEXT[mode]}</CommandEmpty>
        {mode === "requests" && <RequestItems close={close} />}
        {mode === "critters" && <CritterItems close={close} />}
        {mode === "flows" && <FlowItems close={close} />}
      </CommandList>
    </CommandDialog>
  );
}
