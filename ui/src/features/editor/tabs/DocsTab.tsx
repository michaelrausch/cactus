import { Textarea } from "@/components/ui/textarea";
import type { ScenarioTabProps } from "../types";

export function DocsTab({ scenario, onChange }: ScenarioTabProps) {
  return (
    <Textarea
      value={scenario.docs ?? ""}
      onChange={(e) => {
        const { docs: _previous, ...rest } = scenario;
        onChange(e.target.value ? { ...rest, docs: e.target.value } : rest);
      }}
      placeholder="What does this request do?"
      className="min-h-0 flex-1 resize-none rounded-lg"
    />
  );
}
