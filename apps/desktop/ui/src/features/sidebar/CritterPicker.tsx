import { Check, ChevronsUpDown, Pause, Play, Plus, UserRound } from "lucide-react";
import { actAs, openNewCritter } from "@/actions/critters";
import { runFlow } from "@/actions/flows";
import { refreshSchedules, setSchedulesPaused } from "@/actions/monitors";
import type { CritterSummary } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { speciesInfo } from "@/components/critters/species";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { selectActiveCritter, useCritterStore } from "@/stores/critterStore";
import { selectFailingSchedules, useMonitorStore } from "@/stores/monitorStore";

const plural = (count: number, noun: string) => `${count} ${noun}${count === 1 ? "" : "s"}`;

function CritterRow({ critter, active }: { critter: CritterSummary; active: boolean }) {
  return (
    <DropdownMenuItem onSelect={() => void actAs(critter.id)}>
      <CritterSprite
        species={critter.species}
        size={16}
        label={`${critter.name}, ${speciesInfo(critter.species).name}`}
        seed={critter.id}
      />
      <span className="min-w-0 flex-1 truncate">{critter.name}</span>
      {active && <Check className="text-brand-2" />}
    </DropdownMenuItem>
  );
}

/** The active critter's flows, each run with one click. */
function FlowItems({ critter }: { critter: CritterSummary }) {
  if (critter.flows.length === 0) return null;
  return (
    <>
      <DropdownMenuSeparator />
      <DropdownMenuLabel className="text-xs text-muted-foreground">{critter.name}'s flows</DropdownMenuLabel>
      {critter.flows.map((flow) => (
        <DropdownMenuItem
          key={flow.id}
          disabled={!!flow.error}
          title={flow.error ?? undefined}
          onSelect={() => void runFlow(critter.id, flow.id)}
        >
          <Play /> <span className="min-w-0 flex-1 truncate">{flow.name}</span>
        </DropdownMenuItem>
      ))}
    </>
  );
}

/**
 * Footer control for choosing who requests are sent as. Reads "No critter"
 * until one is picked; a red dot means a scheduled flow is failing.
 */
export function CritterPicker() {
  const critters = useCritterStore((s) => s.critters);
  const activeId = useCritterStore((s) => s.activeId);
  const active = useCritterStore(selectActiveCritter);
  const failing = useMonitorStore((s) => selectFailingSchedules(s).length);
  const scheduled = useMonitorStore((s) => s.schedules.length);
  const paused = useMonitorStore((s) => s.paused);
  const alerted = useMonitorStore((s) => s.flashing !== null && s.flashing === activeId);

  const label = active ? `Sending as ${active.name}` : "Sending as no critter";
  return (
    <DropdownMenu onOpenChange={(open) => open && void refreshSchedules()}>
      <DropdownMenuTrigger
        title={label}
        aria-label={label}
        className="relative flex h-8 min-w-0 flex-1 items-center gap-2 rounded-lg px-2 text-left text-sm focus-ring hover:bg-sidebar-accent"
      >
        {active ? (
          <CritterSprite
            species={active.species}
            size={16}
            state={alerted ? "fail" : "idle"}
            label={active.name}
            seed={active.id}
            asleep={paused}
          />
        ) : (
          <UserRound className="size-4 shrink-0 text-muted-foreground" />
        )}
        <span className="min-w-0 flex-1 truncate">{active?.name ?? "No critter"}</span>
        {failing > 0 && (
          <span
            title={`${plural(failing, "scheduled flow")} failing`}
            className="size-2 shrink-0 rounded-full bg-danger"
          >
            <span className="sr-only">{plural(failing, "scheduled flow")} failing</span>
          </span>
        )}
        <ChevronsUpDown className="size-3.5 shrink-0 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" side="top" className="w-60">
        <DropdownMenuLabel className="text-xs text-muted-foreground">Send as</DropdownMenuLabel>
        <DropdownMenuItem onSelect={() => void actAs(null)}>
          <UserRound /> <span className="flex-1">No critter</span>
          {!activeId && <Check className="text-brand-2" />}
        </DropdownMenuItem>
        {critters.map((critter) => (
          <CritterRow key={critter.id} critter={critter} active={critter.id === activeId} />
        ))}
        {active && <FlowItems critter={active} />}
        <DropdownMenuSeparator />
        {scheduled > 0 && (
          <DropdownMenuItem onSelect={() => void setSchedulesPaused(!paused)}>
            {paused ? <Play /> : <Pause />}
            <span className="flex-1">{paused ? "Resume schedules" : "Pause schedules"}</span>
            {failing > 0 && <span className="text-xs text-danger">{failing} failing</span>}
          </DropdownMenuItem>
        )}
        <DropdownMenuItem onSelect={openNewCritter}>
          <Plus /> New critter…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
