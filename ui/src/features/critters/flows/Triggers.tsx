import { useState, type ComponentType } from "react";
import { BellOff, Clock, Play, Plus, Trash2 } from "lucide-react";
import { editFlow } from "@/actions/flows";
import { ensureNotificationPermission, runScheduledNow } from "@/actions/monitors";
import type { AlertOn, CritterSummary, Flow, Schedule, Trigger } from "@/api/client";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { countdown, formatInterval, parseInterval } from "@/domain/interval";
import { useNow } from "@/hooks/useNow";
import { cn } from "@/lib/utils";
import { useMonitorStore } from "@/stores/monitorStore";
import { useProjectStore } from "@/stores/projectStore";

/** The env select's stand-in for "whichever env is active when it runs". */
const ACTIVE_ENV = "__active__";
/** What a flow alerts on without an `alert` key. */
const DEFAULT_ALERTS: readonly AlertOn[] = ["failure"];

type TriggerType = Trigger["type"];

/** The kinds of trigger a flow can have; a new kind adds a row here and a chip below. */
const TRIGGER_KINDS: { type: TriggerType; label: string; icon: ComponentType; create: () => Trigger }[] = [
  { type: "schedule", label: "Schedule", icon: Clock, create: () => ({ type: "schedule", every: "5m" }) },
];

/** `triggers`, left out of the file while empty. */
function withTriggers(flow: Flow, triggers: Trigger[]): Flow {
  const { triggers: _previous, ...rest } = flow;
  return triggers.length > 0 ? { ...rest, triggers } : rest;
}

const editTrigger = (index: number, change: (trigger: Trigger) => Trigger) =>
  editFlow((flow) =>
    withTriggers(
      flow,
      (flow.triggers ?? []).map((t, i) => (i === index ? change(t) : t)),
    ),
  );

const removeTrigger = (index: number) =>
  editFlow((flow) =>
    withTriggers(
      flow,
      (flow.triggers ?? []).filter((_, i) => i !== index),
    ),
  );

/** `alert.on`, left out of the file while it's the default. */
function withAlerts(flow: Flow, on: AlertOn[]): Flow {
  const { alert: _previous, ...rest } = flow;
  const isDefault = on.length === DEFAULT_ALERTS.length && on.every((kind) => DEFAULT_ALERTS.includes(kind));
  return isDefault ? rest : { ...rest, alert: { on } };
}

function IntervalInput({ value, onCommit }: { value: string; onCommit: (every: string) => void }) {
  const [draft, setDraft] = useState(value);
  const valid = parseInterval(draft) !== null;
  const commit = () => {
    const seconds = parseInterval(draft);
    if (seconds === null) return;
    const every = formatInterval(seconds);
    setDraft(every);
    onCommit(every);
  };
  return (
    <Input
      value={draft}
      aria-label="Every"
      aria-invalid={!valid}
      title={valid ? undefined : "e.g. 30s, 5m or 1h; at least 30s"}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => event.key === "Enter" && commit()}
      className="h-8 w-20 font-mono text-xs aria-invalid:border-danger"
    />
  );
}

const ROW = "flex items-center gap-2 text-xs";
const LABEL = "w-16 shrink-0 text-muted-foreground";

interface ScheduleProps {
  critter: CritterSummary;
  flowId: string;
  index: number;
  schedule: Schedule;
  model: Flow;
  editable: boolean;
  /** Opens the popover once, right after the trigger is added. */
  defaultOpen: boolean;
}

/** "Every 5m · next 4:12": click to change how often, where, and what alerts. */
function ScheduleChip({ critter, flowId, index, schedule, model, editable, defaultOpen }: ScheduleProps) {
  const envs = useProjectStore((s) => s.envs);
  const state = useMonitorStore((s) => s.schedules.find((x) => x.critter === critter.id && x.flow === flowId));
  const paused = useMonitorStore((s) => s.paused);
  const notificationsOff = useMonitorStore((s) => s.notifications === "denied");
  // The scheduler runs the saved file, so "next" and "Run now" wait for a save.
  const saved = critter.flows.find((flow) => flow.id === flowId)?.triggers.some((t) => t.type === "schedule");
  const now = useNow(1000);
  const alerts = model.alert?.on ?? DEFAULT_ALERTS;
  const edit = (change: (schedule: Schedule) => Schedule) =>
    editTrigger(index, (trigger) => (trigger.type === "schedule" ? { ...change(trigger), type: "schedule" } : trigger));
  const toggleAlert = (kind: AlertOn, on: boolean) =>
    editFlow((flow) => withAlerts(flow, on ? [...new Set([...alerts, kind])] : alerts.filter((k) => k !== kind)));

  let status: string | null = null;
  if (paused) status = "paused";
  else if (state?.waiting === "secretsLocked") status = "secrets locked";
  else if (state && state.nextAt !== null) status = `next ${countdown(state.nextAt, now)}`;
  const failing = state?.last ? !state.last.ok : false;

  return (
    <Popover defaultOpen={defaultOpen}>
      <PopoverTrigger asChild>
        <Button size="sm" variant="outline" className="h-7 gap-1.5 font-normal">
          <Clock className="text-muted-foreground" />
          Every {schedule.every}
          {status && <span className="text-muted-foreground tabular-nums">· {status}</span>}
          {failing && (
            <span className="size-1.5 rounded-full bg-danger" title="Last scheduled run failed">
              <span className="sr-only">Last scheduled run failed</span>
            </span>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="flex w-80 flex-col gap-3">
        <div className={ROW}>
          <span className={LABEL}>Every</span>
          <fieldset disabled={!editable} className="contents">
            <IntervalInput
              key={schedule.every}
              value={schedule.every}
              onCommit={(every) => edit((s) => ({ ...s, every }))}
            />
          </fieldset>
        </div>
        <div className={ROW}>
          <span className={LABEL}>In</span>
          <Select
            value={schedule.env ?? ACTIVE_ENV}
            disabled={!editable}
            onValueChange={(env) =>
              edit(({ env: _previous, ...rest }) => (env === ACTIVE_ENV ? rest : { ...rest, env }))
            }
          >
            <SelectTrigger className="h-8 flex-1 text-xs" aria-label="Environment">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={ACTIVE_ENV}>Active environment</SelectItem>
              {envs.map((env) => (
                <SelectItem key={env} value={env}>
                  {env}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className={ROW}>
          <span className={LABEL}>Alert on</span>
          {(["failure", "change"] as const).map((kind) => (
            <label key={kind} className="flex items-center gap-1">
              <input
                type="checkbox"
                checked={alerts.includes(kind)}
                disabled={!editable}
                onChange={(event) => toggleAlert(kind, event.target.checked)}
              />
              {kind === "failure" ? "failures" : "changes"}
            </label>
          ))}
        </div>
        {notificationsOff && (
          <p className="flex items-center gap-1 text-xs text-muted-foreground">
            <BellOff className="size-3.5" /> Notifications are off; alerts show in the app.
          </p>
        )}
        {!saved && <p className="text-xs text-muted-foreground">Runs once saved.</p>}
        <div className="flex items-center justify-between border-t pt-2">
          <Button
            size="sm"
            variant="ghost"
            disabled={!saved}
            onClick={() => void runScheduledNow(critter.id, flowId)}
            title="Run now, recorded and alerted on like a scheduled run"
          >
            <Play /> Run now
          </Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={!editable}
            className="text-danger"
            onClick={() => removeTrigger(index)}
          >
            <Trash2 /> Remove
          </Button>
        </div>
      </PopoverContent>
    </Popover>
  );
}

interface TriggersProps {
  critter: CritterSummary;
  flowId: string;
  model: Flow;
  /** Trigger edits go through the visual editor; YAML mode edits the file directly. */
  editable: boolean;
}

/** The flow's triggers as chips in its header, and a way to add one. */
export function Triggers({ critter, flowId, model, editable }: TriggersProps) {
  const triggers = model.triggers ?? [];
  // The kind just added, so its popover opens.
  const [added, setAdded] = useState<TriggerType | null>(null);
  // One of each kind: the scheduler follows one schedule per flow.
  const addable = TRIGGER_KINDS.filter((kind) => !triggers.some((t) => t.type === kind.type));
  const add = (kind: (typeof TRIGGER_KINDS)[number]) => {
    editFlow((flow) => withTriggers(flow, [...(flow.triggers ?? []), kind.create()]));
    setAdded(kind.type);
    void ensureNotificationPermission();
  };
  return (
    <>
      {triggers.map((trigger, index) => {
        switch (trigger.type) {
          case "schedule":
            return (
              <ScheduleChip
                key={trigger.type}
                critter={critter}
                flowId={flowId}
                index={index}
                schedule={trigger}
                model={model}
                editable={editable}
                defaultOpen={added === trigger.type}
              />
            );
        }
      })}
      {addable.length > 0 && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild disabled={!editable}>
            <Button size="sm" variant="ghost" className={cn("h-7 font-normal text-muted-foreground")}>
              <Plus /> Trigger
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {addable.map((kind) => (
              <DropdownMenuItem key={kind.type} onSelect={() => add(kind)}>
                <kind.icon /> {kind.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </>
  );
}
