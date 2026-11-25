import { Globe } from "lucide-react";
import { bind, Fields, TextInput } from "./fields";
import { defineTool } from "./types";

export const open = defineTool<"open">({
  label: "Open in browser",
  icon: Globe,
  create: () => ({ tool: "open" }),
  summary: (step) => step.url ?? "",
  dataflow: (step) => ({
    templates: [step.url ?? ""],
    problems: step.url?.trim() ? [] : ["No URL"],
  }),
  Editor: (props) => (
    <Fields>
      <TextInput label="URL" mono placeholder="https://" {...bind(props, "url")} />
    </Fields>
  ),
});
