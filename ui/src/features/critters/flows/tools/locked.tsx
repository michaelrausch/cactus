import { Lock } from "lucide-react";
import type { ProToolId } from "@/edition/types";
import type { ToolDef, ToolId } from "./types";

const PRO_LABELS: Record<ProToolId, string> = {
  webhook: "Webhook",
  slack: "Slack",
  fuzz: "Fuzz",
  headers: "Security headers",
  cert: "Certificate",
  ratelimit: "Rate limit",
  loadtest: "Load test",
};

/** A Pro tool's step in the free edition: kept as written, not editable, and an error when run. */
export const lockedTool = (id: ToolId): ToolDef<ToolId> => ({
  label: PRO_LABELS[id as ProToolId] ?? id,
  icon: Lock,
  create: () => ({ tool: id }) as never,
  summary: () => "Pro",
  Editor: () => <p className="text-sm text-muted-foreground">Needs cactus pro</p>,
});
