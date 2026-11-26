import type { AlertEvent } from "@/api/client";
import { findingsSummary } from "./findings";

export type AlertTone = "warning" | "success" | "info";

/** What an alert says: `{ title: "Health failing", description: "Step 2 (status.yml): status 500" }`. */
export function alertMessage(
  event: AlertEvent,
  critter: string,
  flow: string,
): { title: string; description: string; tone: AlertTone } {
  switch (event.kind) {
    case "failing":
      return {
        title: `${flow} failing`,
        description:
          event.step === null
            ? `${critter}: ${event.reason}`
            : `Step ${event.step + 1} (${event.request}): ${event.reason}`,
        tone: "warning",
      };
    case "recovered":
      return { title: `${flow} recovered`, description: `${critter}'s flow passes again`, tone: "success" };
    case "changed":
      return {
        title: `${flow}: ${event.expression} changed`,
        description: `${event.before ?? "—"} → ${event.after ?? "—"}`,
        tone: "info",
      };
    case "newFindings":
      return {
        title: `${flow}: new findings`,
        description: `Step ${event.step + 1} (${event.request}): ${findingsSummary(event.findings)}`,
        tone: "warning",
      };
  }
}
