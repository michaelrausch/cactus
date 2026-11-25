import { backend, type FlowAsked, type FlowDisplayed } from "@/api/client";
import { reportErrors } from "@/lib/errors";
import { flowStore } from "./shared";

/** A running flow asks something (the `FlowAsked` event): show it. */
export function flowAsked(ask: FlowAsked): void {
  flowStore().asked(ask);
}

/** A running flow shows a message (the `FlowDisplayed` event); the run carries on. */
export function flowDisplayed(message: FlowDisplayed): void {
  flowStore().displayed(message);
}

export function dismissFlowMessage(): void {
  flowStore().displayed(null);
}

/** The run stopped waiting, e.g. it was cancelled (the `FlowAskClosed` event). */
export function flowAskClosed(id: number): void {
  flowStore().askClosed(id);
}

/** Answer the question showing: its values, or `null` to stop the run. */
export async function answerFlowAsk(values: Record<string, string> | null): Promise<void> {
  const { ask } = flowStore();
  if (!ask) return;
  flowStore().askClosed(ask.id);
  await reportErrors(() => backend.answerFlowAsk(ask.id, values), "Couldn't answer");
}
