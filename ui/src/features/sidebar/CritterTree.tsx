import { useState, type MouseEvent, type ReactNode } from "react";
import { ChevronRight, Clock, MoreHorizontal, Play, Plus, Star, TriangleAlert } from "lucide-react";
import {
  actAs,
  deleteCritter,
  duplicateCritter,
  openNewCritter,
  renameCritterFolder,
  showCritter,
} from "@/actions/critters";
import {
  createFlow,
  deleteFlow,
  duplicateFlow,
  renameFlow,
  runFlow,
  setLoginFlow,
  switchToFlow,
} from "@/actions/flows";
import type { CritterSummary, FlowSummary } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { speciesInfo } from "@/components/critters/species";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { expiredTokens } from "@/domain/jwt";
import { useNow } from "@/hooks/useNow";
import { cn } from "@/lib/utils";
import { useCritterStore } from "@/stores/critterStore";
import { useFlowStore } from "@/stores/flowStore";
import { useMonitorStore } from "@/stores/monitorStore";
import { useUiStore } from "@/stores/uiStore";

const plural = (count: number, noun: string) => `${count} ${noun}${count === 1 ? "" : "s"}`;

const ROW = "group relative flex h-8 items-center rounded-lg pr-1 transition-colors hover:bg-sidebar-accent/60";
const ROW_SELECTED = "bg-sidebar-accent hover:bg-sidebar-accent";
const ROW_BUTTON = "flex h-full min-w-0 flex-1 cursor-default items-center gap-2 rounded-lg text-left focus-ring";
const ACTION =
  "grid size-6 shrink-0 place-items-center rounded-md text-muted-foreground focus-ring hover:bg-sidebar-accent hover:text-foreground";

function SelectedBar({ selected }: { selected: boolean }) {
  return selected ? <span className="absolute top-2 bottom-2 left-0 w-0.5 rounded-full bg-brand" /> : null;
}

/** ⋯ on hover, and the same menu on right-click. */
function RowMenu({
  label,
  open,
  onOpenChange,
  children,
}: {
  label: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: ReactNode;
}) {
  return (
    <DropdownMenu open={open} onOpenChange={onOpenChange}>
      <DropdownMenuTrigger
        aria-label={label}
        className={cn(
          ACTION,
          "opacity-0 group-hover:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100",
        )}
      >
        <MoreHorizontal className="size-3.5" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">{children}</DropdownMenuContent>
    </DropdownMenu>
  );
}

function useContextMenu() {
  const [open, setOpen] = useState(false);
  const onContextMenu = (event: MouseEvent) => {
    event.preventDefault();
    setOpen(true);
  };
  return { open, setOpen, onContextMenu };
}

/** Green or red for a flow's last scheduled run on this machine; the critter's sprite while it runs here. */
function FlowState({ critter, flow }: { critter: CritterSummary; flow: FlowSummary }) {
  const last = useMonitorStore((s) => s.schedules.find((x) => x.critter === critter.id && x.flow === flow.id)?.last);
  const running = useFlowStore((s) => s.run.running && s.open?.critter === critter.id && s.open.flow === flow.id);
  if (running)
    return <CritterSprite species={critter.species} size={16} state="work" label="Running" seed={critter.id} />;
  if (!last) return null;
  const text = last.ok ? "Last scheduled run passed" : "Last scheduled run failed";
  return (
    <span title={text} className={cn("size-1.5 shrink-0 rounded-full", last.ok ? "bg-success" : "bg-danger")}>
      <span className="sr-only">{text}</span>
    </span>
  );
}

function FlowRow({ critter, flow }: { critter: CritterSummary; flow: FlowSummary }) {
  const menu = useContextMenu();
  // A flow stays open in the store while the main pane shows something else.
  const selected = useFlowStore((s) => s.open?.critter === critter.id && s.open.flow === flow.id);
  const showing = useUiStore((s) => s.view === "flow") && selected;
  const isLogin = flow.id === critter.login;
  const scheduled = flow.triggers.length > 0;
  return (
    <div onContextMenu={menu.onContextMenu} className={cn(ROW, showing && ROW_SELECTED)}>
      <SelectedBar selected={showing} />
      <button
        type="button"
        onClick={() => void switchToFlow(critter.id, flow.id)}
        title={flow.error ?? `critters/${critter.id}/flows/${flow.id}.yml`}
        aria-current={showing || undefined}
        className={cn(ROW_BUTTON, "pl-8")}
      >
        {scheduled ? (
          <Clock className="size-3.5 shrink-0 text-muted-foreground" aria-label="Has a trigger" />
        ) : (
          <span className="size-3.5 shrink-0" />
        )}
        <span
          className={cn("min-w-0 flex-1 truncate text-sm", showing ? "text-foreground" : "text-sidebar-foreground/90")}
        >
          {flow.name}
        </span>
        {isLogin && <Star className="size-3 shrink-0 fill-warning text-warning" aria-label="Login flow" />}
        {flow.error && <TriangleAlert className="size-3.5 shrink-0 text-danger" aria-label="Doesn't parse" />}
        <FlowState critter={critter} flow={flow} />
      </button>
      {!flow.error && (
        <button
          type="button"
          title={`Run ${flow.name}`}
          aria-label={`Run ${flow.name}`}
          onClick={() => void runFlow(critter.id, flow.id)}
          className={cn(ACTION, "opacity-0 group-hover:opacity-100 focus-visible:opacity-100")}
        >
          <Play className="size-3.5" />
        </button>
      )}
      <RowMenu label={`More for ${flow.name}`} open={menu.open} onOpenChange={menu.setOpen}>
        <DropdownMenuItem disabled={!!flow.error} onSelect={() => void runFlow(critter.id, flow.id)}>
          Run
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => renameFlow(critter.id, flow.id)}>Rename file…</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => duplicateFlow(critter.id, flow.id, flow.name)}>Duplicate…</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => void setLoginFlow(critter.id, isLogin ? null : flow.id)}>
          {isLogin ? "Stop using as login flow" : "Set as login flow"}
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className="text-danger" onSelect={() => void deleteFlow(critter.id, flow.id)}>
          Delete
        </DropdownMenuItem>
      </RowMenu>
    </div>
  );
}

/** What needs attention: "token expired · 2 missing secrets", or null. */
function useProblems(critter: CritterSummary): string | null {
  const context = useCritterStore((s) => s.contexts[critter.id]);
  const failing = useMonitorStore((s) => s.schedules.some((x) => x.critter === critter.id && x.last && !x.last.ok));
  const now = useNow(30_000);
  const expired = expiredTokens(context ?? {}, now).length > 0;
  const problems = [
    critter.error && "critter.yml has an error",
    expired && "token expired",
    critter.missingSecrets.length > 0 && plural(critter.missingSecrets.length, "missing secret"),
    failing && "failing schedule",
  ].filter(Boolean);
  return problems.length > 0 ? problems.join(" · ") : null;
}

interface CritterRowProps {
  critter: CritterSummary;
  expanded: boolean;
  onToggle: () => void;
}

function CritterRow({ critter, expanded, onToggle }: CritterRowProps) {
  const menu = useContextMenu();
  const onCritterView = useUiStore((s) => s.view === "critter");
  const selected = useCritterStore((s) => s.selectedId === critter.id);
  const showing = onCritterView && selected;
  const active = useCritterStore((s) => s.activeId === critter.id);
  const alerted = useMonitorStore((s) => s.flashing === critter.id);
  const problems = useProblems(critter);
  const species = speciesInfo(critter.species);
  return (
    <div onContextMenu={menu.onContextMenu} className={cn(ROW, showing && ROW_SELECTED)}>
      <SelectedBar selected={showing} />
      <button
        type="button"
        aria-label={expanded ? `Collapse ${critter.name}` : `Expand ${critter.name}`}
        aria-expanded={expanded}
        onClick={onToggle}
        className="grid size-6 shrink-0 place-items-center rounded-md text-muted-foreground focus-ring"
      >
        <ChevronRight className={cn("size-3.5 transition-transform", expanded && "rotate-90")} />
      </button>
      <button
        type="button"
        onClick={() => showCritter(critter.id)}
        aria-current={showing || undefined}
        className={ROW_BUTTON}
      >
        <CritterSprite
          species={critter.species}
          size={16}
          state={alerted ? "fail" : "idle"}
          label={`${critter.name}, ${species.name}`}
          seed={critter.id}
        />
        <span
          className={cn("min-w-0 flex-1 truncate text-sm", showing ? "text-foreground" : "text-sidebar-foreground/90")}
        >
          {critter.name}
        </span>
        {active && <span className="shrink-0 text-2xs text-muted-foreground">sending as</span>}
        {problems && (
          <span title={problems} className="size-1.5 shrink-0 rounded-full bg-warning">
            <span className="sr-only">{problems}</span>
          </span>
        )}
      </button>
      <button
        type="button"
        title="New flow"
        aria-label={`New flow for ${critter.name}`}
        onClick={() => createFlow(critter.id)}
        className={cn(ACTION, "opacity-0 group-hover:opacity-100 focus-visible:opacity-100")}
      >
        <Plus className="size-3.5" />
      </button>
      <RowMenu label={`More for ${critter.name}`} open={menu.open} onOpenChange={menu.setOpen}>
        <DropdownMenuItem onSelect={() => void actAs(active ? null : critter.id)}>
          {active ? "Stop sending as" : `Send as ${critter.name}`}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => createFlow(critter.id)}>New flow…</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => duplicateCritter(critter.id)}>Duplicate…</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => renameCritterFolder(critter.id)}>Rename folder…</DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem className="text-danger" onSelect={() => deleteCritter(critter.id)}>
          Delete…
        </DropdownMenuItem>
      </RowMenu>
    </div>
  );
}

const matches = (text: string, filter: string) => text.toLowerCase().includes(filter.toLowerCase());

/** The critters and, under each, its flows. The search filters them too. */
export function CritterTree({ filter }: { filter: string }) {
  const critters = useCritterStore((s) => s.critters);
  const selectedId = useCritterStore((s) => s.selectedId);
  const openCritter = useFlowStore((s) => s.open?.critter);
  // Collapsed or expanded by hand; otherwise the one being shown is expanded.
  const [toggled, setToggled] = useState<Record<string, boolean>>({});
  const query = filter.trim();

  // A critter whose name matches keeps all its flows; otherwise only matching flows show.
  const shown = critters
    .map((critter) => {
      if (!query || matches(critter.name, query)) return { critter, flows: critter.flows };
      return { critter, flows: critter.flows.filter((flow) => matches(flow.name, query)) };
    })
    .filter(({ critter, flows }) => !query || flows.length > 0 || matches(critter.name, query));
  if (query && shown.length === 0) return null;

  return (
    <section aria-label="Critters" className="mt-3">
      <div className="group flex h-6 items-center gap-1 pr-1 pl-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
        <span className="flex-1">Critters</span>
        <button
          type="button"
          title="New critter"
          aria-label="New critter"
          onClick={openNewCritter}
          className={cn(ACTION, "size-6")}
        >
          <Plus className="size-3.5" />
        </button>
      </div>
      {shown.map(({ critter, flows }) => {
        const expanded = !!query || (toggled[critter.id] ?? (critter.id === selectedId || critter.id === openCritter));
        return (
          <div key={critter.id}>
            <CritterRow
              critter={critter}
              expanded={expanded}
              onToggle={() => setToggled((t) => ({ ...t, [critter.id]: !expanded }))}
            />
            {expanded && flows.map((flow) => <FlowRow key={flow.id} critter={critter} flow={flow} />)}
            {expanded && !query && flows.length === 0 && (
              <button
                type="button"
                onClick={() => createFlow(critter.id)}
                className="flex h-7 w-full cursor-default items-center gap-2 rounded-lg pl-8 text-left text-sm text-muted-foreground focus-ring hover:bg-sidebar-accent/60 hover:text-foreground"
              >
                <Plus className="size-3.5" /> New flow
              </button>
            )}
          </div>
        );
      })}
    </section>
  );
}
