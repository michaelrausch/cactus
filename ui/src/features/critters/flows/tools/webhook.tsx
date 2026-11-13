import { Webhook } from "lucide-react";
import { bind, Fields, MapInput, TextInput } from "./fields";
import { defineTool } from "./types";

export const webhook = defineTool<"webhook">({
  label: "Webhook",
  icon: Webhook,
  create: () => ({ tool: "webhook" }),
  summary: (step) => `${step.method?.trim().toUpperCase() || "POST"} ${step.url ?? ""}`,
  dataflow: (step) => ({
    templates: [step.url ?? "", step.body ?? "", ...Object.values(step.headers ?? {})],
    problems: step.url?.trim() ? [] : ["No URL"],
  }),
  Editor: (props) => (
    <Fields>
      <TextInput label="URL" mono placeholder="https://" {...bind(props, "url")} />
      <TextInput label="Method" mono templates={false} placeholder="POST" {...bind(props, "method")} />
      <MapInput label="Headers" keyPlaceholder="Header" {...bind(props, "headers")} />
      <TextInput label="Body" multiline json mono {...bind(props, "body")} />
    </Fields>
  ),
});
