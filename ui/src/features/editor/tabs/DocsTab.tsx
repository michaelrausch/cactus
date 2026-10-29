import { Textarea } from "@/components/ui/textarea";
import type { ScenarioTabProps } from "../types";

export function DocsTab({ scenario, onChange }: ScenarioTabProps) {
  return (
    <Textarea
      value={scenario.description}
      onChange={(e) => onChange({ ...scenario, description: e.target.value })}
      placeholder="What does this request do?"
      className="min-h-0 flex-1 resize-none rounded-lg"
    />
  );
}
