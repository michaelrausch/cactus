import { TextQuote } from "lucide-react";
import { bind, Checkbox, Fields, TextInput } from "./fields";
import { defineTool } from "./types";

export const display = defineTool<"display">({
  label: "Display",
  icon: TextQuote,
  create: () => ({ tool: "display" }),
  summary: (step) => step.title || step.text || "",
  dataflow: (step) => ({ templates: [step.title ?? "", step.text ?? ""] }),
  Editor: (props) => (
    <Fields>
      <TextInput label="Title" placeholder="Display" {...bind(props, "title")} />
      <TextInput label="Text" multiline {...bind(props, "text")} />
      <Checkbox label="Pause" {...bind(props, "pause")} />
    </Fields>
  ),
});
