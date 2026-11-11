import { useState, type MouseEvent } from "react";
import { MoreHorizontal, Play, Plus, Star, TriangleAlert } from "lucide-react";
import { runFlowNow } from "@/actions/critters";
import { createFlow, deleteFlow, duplicateFlow, openFlow, renameFlow, setLoginFlow } from "@/actions/flows";
import type { CritterSummary, FlowSummary } from "@/api/client";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useMonitorStore } from "@/stores/monitorStore";

function LastRunDot({ critter, flow }: { critter: string; flow: string }) {
  const last = useMonitorStore((s) => s.schedules.find((x) => x.critter === critter && x.flow === flow)?.last);
  if (!last) return <span className="size-2 shrink-0" />;
  return (
    <span
      title={last.ok ? "Last scheduled run passed" : "Last scheduled run failed"}
      className={cn("size-2 shrink-0 rounded-full", last.ok ? "bg-success" : "bg-danger")}
    />
  );
}

interface RowProps {
  critter: CritterSummary;
  flow: FlowSummary;
}

/** One flow: click to edit, ▶ to run, right-click or ⋯ for the rest. */
function FlowRow({ critter, flow }: RowProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const isLogin = flow.id === critter.login;
  const openMenu = (event: MouseEvent) => {
    event.preventDefault();
    setMenuOpen(true);
  };
  return (
    <li onContextMenu={openMenu} className="group flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-accent/40">
      <LastRunDot critter={critter.id} flow={flow.id} />
      <button
        type="button"
        onClick={() => void openFlow(critter.id, flow.id)}
        className="flex min-w-0 flex-1 flex-col rounded text-left focus-ring"
      >
        <span className="flex items-center gap-1.5 truncate text-sm font-medium">
          {flow.name}
          {isLogin && <Star className="size-3.5 shrink-0 fill-warning text-warning" aria-label="Login flow" />}
        </span>
        <span className="truncate font-mono text-xs text-muted-foreground">flows/{flow.id}.yml</span>
      </button>
      {flow.error ? (
        <span className="flex items-center gap-1 text-xs text-danger" title={flow.error}>
          <TriangleAlert className="size-3.5" /> doesn't parse
        </span>
      ) : (
        <span className="text-xs text-muted-foreground tabular-nums">
          {flow.steps} {flow.steps === 1 ? "step" : "steps"}
          {flow.trigger && ` · ⏱ ${flow.trigger.every}`}
        </span>
      )}
      <Button
        size="sm"
        variant="ghost"
        disabled={!!flow.error}
        aria-label={`Run ${flow.name}`}
        onClick={() => void runFlowNow(critter.id, flow.id)}
      >
        <Play />
      </Button>
      <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
        <DropdownMenuTrigger
          aria-label={`More for ${flow.name}`}
          className="grid size-6 place-items-center rounded-md text-muted-foreground opacity-0 focus-ring group-hover:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
        >
          <MoreHorizontal className="size-4" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onSelect={() => renameFlow(critter.id, flow.id)}>Rename file…</DropdownMenuItem>
          <DropdownMenuItem onSelect={() => duplicateFlow(critter.id, flow.id, flow.name)}>Duplicate…</DropdownMenuItem>
          <DropdownMenuItem onSelect={() => void setLoginFlow(critter.id, isLogin ? null : flow.id)}>
            {isLogin ? "Stop using as login flow" : "Set as login flow"}
          </DropdownMenuItem>
          <DropdownMenuSeparator />
          <DropdownMenuItem className="text-danger" onSelect={() => void deleteFlow(critter.id, flow.id)}>
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  );
}

/** The critter's flows, and a way to make one. */
export function FlowList({ critter }: { critter: CritterSummary }) {
  return (
    <div className="flex flex-col gap-2 p-3">
      {critter.flows.length === 0 ? (
        <p className="px-2 py-1 text-sm text-muted-foreground">
          No flows yet. A flow is a list of requests that {critter.name} sends in order, passing values like tokens from
          one to the next.
        </p>
      ) : (
        <ul className="flex flex-col">
          {critter.flows.map((flow) => (
            <FlowRow key={flow.id} critter={critter} flow={flow} />
          ))}
        </ul>
      )}
      <Button variant="ghost" size="sm" className="self-start" onClick={() => createFlow(critter.id)}>
        <Plus /> New flow
      </Button>
    </div>
  );
}
