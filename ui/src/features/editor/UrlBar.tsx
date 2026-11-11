import { ArrowUp, Square } from "lucide-react";
import { cancelInFlight, sendCurrentScenario } from "@/actions/request";
import { withShortcut } from "@/app/shortcuts";
import { CritterSprite } from "@/components/critters/CritterSprite";
import { MethodBadge } from "@/components/MethodBadge";
import { TemplateField } from "@/components/template/TemplateField";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { HTTP_METHODS } from "@/domain/http";
import { withUrl } from "@/domain/scenario";
import { cn } from "@/lib/utils";
import { methodColor } from "@/theme/httpColors";
import { selectActiveCritter, useCritterStore } from "@/stores/critterStore";
import { useResponseStore } from "@/stores/responseStore";
import type { ScenarioTabProps } from "./types";

/** Who the request goes out as: the active critter, running while it sends. */
function SenderSprite() {
  const sending = useResponseStore((s) => s.sending);
  const critter = useCritterStore(selectActiveCritter);
  if (!critter) return null;
  return (
    <CritterSprite
      species={critter.species}
      size={16}
      state={sending ? "work" : "idle"}
      label={sending ? `${critter.name} is sending` : `Sending as ${critter.name}`}
      seed={critter.id}
      className="mx-1.5"
    />
  );
}

function SendButton() {
  const sending = useResponseStore((s) => s.sending);
  const critter = useCritterStore(selectActiveCritter);
  const sendLabel = critter ? `Send as ${critter.name}` : "Send";
  if (sending) {
    return (
      <button
        type="button"
        onClick={() => void cancelInFlight()}
        title="Cancel request"
        aria-label="Cancel request"
        className="grid size-9 shrink-0 place-items-center rounded-lg bg-secondary text-foreground focus-ring hover:bg-accent"
      >
        <Square className="size-3.5 animate-pulse fill-current" />
      </button>
    );
  }
  return (
    <button
      type="button"
      onClick={() => void sendCurrentScenario()}
      title={withShortcut(sendLabel, "send")}
      aria-label={sendLabel}
      className="grid size-9 shrink-0 place-items-center rounded-lg bg-brand-2 text-brand-foreground shadow-md shadow-brand-2/20 focus-ring transition hover:brightness-110 active:scale-95"
    >
      <ArrowUp className="size-4" strokeWidth={2.5} />
    </button>
  );
}

export function UrlBar({ scenario, onChange }: ScenarioTabProps) {
  return (
    <div className="flex items-center gap-1 rounded-xl border bg-card p-1 shadow-sm shadow-shadow/20 transition-colors focus-within:border-brand-2/40">
      <Select
        value={scenario.http.method.toUpperCase()}
        onValueChange={(method) => onChange({ ...scenario, http: { ...scenario.http, method } })}
      >
        <SelectTrigger
          className={cn(
            "h-9 w-[6.5rem] border-0 bg-transparent font-mono text-xs font-bold shadow-none dark:bg-transparent dark:hover:bg-accent",
            methodColor(scenario.http.method),
          )}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {HTTP_METHODS.map((method) => (
            <SelectItem key={method} value={method}>
              <MethodBadge method={method} className="text-xs" />
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <TemplateField
        value={scenario.http.url}
        onChange={(url) => onChange(withUrl(scenario, url))}
        placeholder="{{baseUrl}}/path"
        aria-label="URL"
        className="flex-1"
        fieldClassName="h-9 px-2 font-mono text-sm"
      />
      <SenderSprite />
      <SendButton />
    </div>
  );
}
