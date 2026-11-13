import { Hash } from "lucide-react";
import { bind, Fields, TextInput } from "./fields";
import { defineTool } from "./types";

export const slack = defineTool<"slack">({
  label: "Slack",
  icon: Hash,
  create: () => ({ tool: "slack", webhook: "{{SLACK_WEBHOOK}}" }),
  summary: (step) => step.text ?? "",
  dataflow: (step) => ({
    templates: [step.webhook ?? "", step.text ?? ""],
    problems: step.webhook?.trim() ? [] : ["No webhook URL"],
  }),
  Editor: (props) => (
    <Fields>
      <TextInput label="Webhook" mono {...bind(props, "webhook")} />
      <TextInput label="Message" multiline {...bind(props, "text")} />
    </Fields>
  ),
});
