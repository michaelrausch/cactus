import { CircleCheck, CircleX } from "lucide-react";
import type { Check } from "@/api/client";

export function CheckIcon({ ok, className }: { ok: boolean; className: string }) {
  return ok ? (
    <CircleCheck className={`${className} text-emerald-400`} />
  ) : (
    <CircleX className={`${className} text-rose-400`} />
  );
}

export function ChecksList({ checks }: { checks: Check[] }) {
  return (
    <div className="flex flex-col gap-1">
      {checks.map((check) => (
        <div key={check.name} className="flex items-start gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-accent/50">
          <CheckIcon ok={check.ok} className="mt-px size-3.5 shrink-0" />
          <span className="font-mono">{check.name}</span>
          <span className="ml-auto text-right text-muted-foreground">{check.detail}</span>
        </div>
      ))}
    </div>
  );
}
