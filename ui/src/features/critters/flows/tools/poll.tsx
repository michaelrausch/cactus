import { RefreshCw } from "lucide-react";
import { expressionSuggester } from "@/domain/suggestions";
import { useProjectStore } from "@/stores/projectStore";
import { bind, ChecksInput, ChoiceInput, Fields, MapInput, NumberInput } from "./fields";
import { defineTool, type ToolEditorProps } from "./types";

/** Mirrors `poll::DEFAULT_EVERY` and `DEFAULT_TIMEOUT`, and `wait::MAX_SECONDS`, in Rust. */
const DEFAULT_EVERY = 2;
const DEFAULT_TIMEOUT = 60;
const MAX_SECONDS = 600;

/** `res.status`, `res.body…`: what a check on the polled response can read. */
const RESPONSE = expressionSuggester(null);

function PollEditor(props: ToolEditorProps<"poll">) {
  const entries = useProjectStore((s) => s.entries);
  const requests = entries.map((entry) => ({ value: entry.relPath, label: entry.title }));
  return (
    <Fields>
      <ChoiceInput label="Request" none="Choose…" options={requests} {...bind(props, "request")} />
      <ChecksInput label="Until" keyPlaceholder="res.body.status" suggest={RESPONSE} {...bind(props, "until")} />
      <div className="flex gap-3">
        <NumberInput label="Every (s)" placeholder={String(DEFAULT_EVERY)} {...bind(props, "every")} />
        <NumberInput
          label="Give up after (s)"
          placeholder={String(DEFAULT_TIMEOUT)}
          max={MAX_SECONDS}
          {...bind(props, "timeout")}
        />
      </div>
      <MapInput label="With" keyPlaceholder="Variable" {...bind(props, "with")} />
      <MapInput label="Capture" keyPlaceholder="Variable" valuePlaceholder="res.body.id" {...bind(props, "capture")} />
    </Fields>
  );
}

export const poll = defineTool<"poll">({
  label: "Poll",
  icon: RefreshCw,
  create: () => ({ tool: "poll" }),
  summary: (step) => step.request ?? "",
  dataflow: (step) => ({
    templates: [...Object.values(step.with ?? {}), ...(step.until ?? []).map((c) => c.value ?? "")],
    sets: Object.keys(step.capture ?? {}),
    problems: [
      ...(step.request?.trim() ? [] : ["No request chosen"]),
      ...((step.until ?? []).length === 0 ? ["No Until checks"] : []),
      ...((step.timeout ?? 0) > MAX_SECONDS ? [`Poll for at most ${MAX_SECONDS} seconds`] : []),
    ],
  }),
  Editor: PollEditor,
});
