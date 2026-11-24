import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import { MoreHorizontal, Plus } from "lucide-react";
import {
  actAs,
  deleteCritter,
  duplicateCritter,
  openNewCritter,
  renameCritterFolder,
  showCritter,
} from "@/actions/critters";
import type { CritterSummary, ScheduleState } from "@/api/client";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { SPECIES } from "@/components/critters/species";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { countdown } from "@/domain/interval";
import { expiredTokens } from "@/domain/jwt";
import { cn } from "@/lib/utils";
import { useCritterStore } from "@/stores/critterStore";
import { useMonitorStore } from "@/stores/monitorStore";

const plural = (count: number, noun: string) => `${count} ${noun}${count === 1 ? "" : "s"}`;

/** The current time, refreshed every `ms`. */
function useNow(ms: number): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), ms);
    return () => clearInterval(timer);
  }, [ms]);
  return now;
}

interface Status {
  text: string;
  problem: boolean;
}

/** "acting as · token expired · 2 missing secrets · failing schedule", or the flow count. */
function statusOf(critter: CritterSummary, active: boolean, expired: boolean, failing: boolean): Status {
  const parts = [
    active && "acting as",
    critter.error && "critter.yml has an error",
    expired && "token expired",
    critter.missingSecrets.length > 0 && plural(critter.missingSecrets.length, "missing secret"),
    failing && "failing schedule",
  ].filter(Boolean);
  const problem = !!critter.error || expired || critter.missingSecrets.length > 0 || failing;
  return parts.length > 0
    ? { text: parts.join(" · "), problem }
    : { text: plural(critter.flows.length, "flow"), problem: false };
}

interface RowProps {
  critter: CritterSummary;
  selected: boolean;
  status: Status;
  menuOpen: boolean;
  onMenuOpenChange: (open: boolean) => void;
  register: (button: HTMLButtonElement | null) => void;
}

/** One critter. Right-click or ⋯ for Act as, Duplicate, Rename and Delete. */
function CritterRow({ critter, selected, status, menuOpen, onMenuOpenChange, register }: RowProps) {
  const openMenu = (event: MouseEvent) => {
    event.preventDefault();
    showCritter(critter.id);
    onMenuOpenChange(true);
  };
  return (
    <div
      onContextMenu={openMenu}
      className={cn("group flex items-center rounded-lg hover:bg-accent/50", selected && "bg-accent/70")}
    >
      <button
        ref={register}
        type="button"
        role="option"
        aria-selected={selected}
        tabIndex={selected ? 0 : -1}
        onClick={() => showCritter(critter.id)}
        className="flex min-w-0 flex-1 items-center gap-3 rounded-lg p-2 text-left focus-ring"
      >
        <CritterSprite
          species={critter.species}
          size={32}
          label={`${critter.name}, ${SPECIES[critter.species].name}${status.problem ? `, ${status.text}` : ""}`}
          seed={critter.id}
        />
        <span className="flex min-w-0 flex-col">
          <span className="truncate text-sm font-medium">{critter.name}</span>
          <span className={cn("truncate text-xs text-muted-foreground", status.problem && "text-warning")}>
            {status.text}
          </span>
        </span>
      </button>
      <DropdownMenu open={menuOpen} onOpenChange={onMenuOpenChange}>
        <DropdownMenuTrigger
          aria-label={`More for ${critter.name}`}
          className="mr-1 grid size-6 place-items-center rounded-md text-muted-foreground opacity-0 focus-ring group-hover:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
        >
          <MoreHorizontal className="size-4" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem onSelect={() => void actAs(critter.id)}>Act as {critter.name}</DropdownMenuItem>
          <DropdownMenuItem onSelect={() => duplicateCritter(critter.id)}>Duplicate…</DropdownMenuItem>
          <DropdownMenuItem onSelect={() => renameCritterFolder(critter.id)}>Rename folder…</DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem className="text-danger" onSelect={() => deleteCritter(critter.id)}>
            Delete…
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

const STATE_DOTS = { ok: "bg-success", failing: "bg-danger", never: "bg-muted-foreground/50" } as const;

/** Every schedule enabled on this machine, with its state and next run. */
function ScheduledList() {
  const schedules = useMonitorStore((s) => s.schedules);
  const paused = useMonitorStore((s) => s.paused);
  const critters = useCritterStore((s) => s.critters);
  const now = useNow(1000);
  if (schedules.length === 0) return null;
  const nameOf = (id: string) => critters.find((c) => c.id === id)?.name ?? id;
  const flowName = (schedule: ScheduleState) =>
    critters.find((c) => c.id === schedule.critter)?.flows.find((f) => f.id === schedule.flow)?.name ?? schedule.flow;
  return (
    <section className="mt-auto flex flex-col gap-1 border-t pt-2">
      <h3 className="px-2 text-xs font-medium tracking-wide text-muted-foreground uppercase">
        Scheduled{paused && " · paused"}
      </h3>
      {schedules.map((schedule) => {
        let state: keyof typeof STATE_DOTS = "never";
        if (schedule.last) state = schedule.last.ok ? "ok" : "failing";
        let next = "not running";
        if (paused) next = "paused";
        else if (schedule.nextAt !== null) next = countdown(schedule.nextAt, now);
        return (
          <button
            key={`${schedule.critter}/${schedule.flow}`}
            type="button"
            onClick={() => showCritter(schedule.critter)}
            className="flex items-center gap-2 rounded-md px-2 py-1 text-left text-xs focus-ring hover:bg-accent/50"
          >
            <span className={cn("size-2 shrink-0 rounded-full", STATE_DOTS[state])}>
              <span className="sr-only">{state}</span>
            </span>
            <span className="min-w-0 flex-1 truncate">
              {nameOf(schedule.critter)} › {flowName(schedule)}
            </span>
            <span className="shrink-0 text-muted-foreground tabular-nums">
              {schedule.every && `⏱ ${schedule.every} · `}
              {next}
            </span>
          </button>
        );
      })}
    </section>
  );
}

/** The critters, ↑/↓ to move, Enter to act as one, then every enabled schedule. */
export function CritterList() {
  const critters = useCritterStore((s) => s.critters);
  const activeId = useCritterStore((s) => s.activeId);
  const selectedId = useCritterStore((s) => s.selectedId);
  const contexts = useCritterStore((s) => s.contexts);
  const schedules = useMonitorStore((s) => s.schedules);
  const [menuFor, setMenuFor] = useState<string | null>(null);
  const buttons = useRef(new Map<string, HTMLButtonElement>());
  const now = useNow(30_000);

  const onKeyDown = (event: KeyboardEvent) => {
    const index = critters.findIndex((c) => c.id === selectedId);
    let next: CritterSummary | undefined;
    if (event.key === "ArrowDown") next = critters[Math.min(index + 1, critters.length - 1)];
    else if (event.key === "ArrowUp") next = critters[Math.max(index - 1, 0)];
    else if (event.key === "Enter" && selectedId) {
      event.preventDefault();
      void actAs(selectedId);
      return;
    } else return;
    event.preventDefault();
    if (!next) return;
    showCritter(next.id);
    buttons.current.get(next.id)?.focus();
  };

  return (
    <nav aria-label="Critters" className="flex w-64 shrink-0 flex-col gap-1 overflow-y-auto border-r p-2">
      <div role="listbox" aria-label="Critters" className="flex flex-col gap-1" onKeyDown={onKeyDown}>
        {critters.map((critter) => {
          const failing = schedules.some((s) => s.critter === critter.id && s.last && !s.last.ok);
          const expired = expiredTokens(contexts[critter.id] ?? {}, now).length > 0;
          return (
            <CritterRow
              key={critter.id}
              critter={critter}
              selected={critter.id === selectedId}
              status={statusOf(critter, critter.id === activeId, expired, failing)}
              menuOpen={menuFor === critter.id}
              onMenuOpenChange={(open) => setMenuFor(open ? critter.id : null)}
              register={(button) => {
                if (button) buttons.current.set(critter.id, button);
                else buttons.current.delete(critter.id);
              }}
            />
          );
        })}
      </div>
      <Button variant="ghost" size="sm" className="mt-1 justify-start" onClick={openNewCritter}>
        <Plus /> New critter
      </Button>
      <ScheduledList />
    </nav>
  );
}
