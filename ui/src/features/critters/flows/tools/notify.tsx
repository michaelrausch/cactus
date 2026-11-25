import { Bell } from "lucide-react";
import { bind, Fields, TextInput } from "./fields";
import { defineTool } from "./types";

export const notify = defineTool<"notify">({
  label: "Notify",
  icon: Bell,
  create: () => ({ tool: "notify" }),
  summary: (step) => step.title ?? "",
  dataflow: (step) => ({ templates: [step.title ?? "", step.body ?? ""] }),
  Editor: (props) => (
    <Fields>
      <TextInput label="Title" {...bind(props, "title")} />
      <TextInput label="Body" multiline {...bind(props, "body")} />
    </Fields>
  ),
});
