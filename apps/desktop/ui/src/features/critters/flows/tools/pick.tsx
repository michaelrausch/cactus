import { ListChecks } from "lucide-react";
import { setField } from "@/domain/flowEdit";
import { bind, Fields, MapInput, TextInput } from "./fields";
import { defineTool, type ToolEditorProps, type ToolOf } from "./types";

/** Variable → path in the chosen item, from `set` and the older `name`/`value`. Mirrors `Pick::outputs` in Rust. */
function outputs(step: ToolOf<"pick">): Record<string, string> {
  const set = { ...step.set };
  const name = step.name?.trim();
  if (name && !(name in set)) set[name] = step.value ?? "";
  return set;
}

function PickEditor(props: ToolEditorProps<"pick">) {
  const { step, update } = props;
  return (
    <Fields>
      <TextInput label="Title" placeholder="Pick" {...bind(props, "title")} />
      <TextInput label="List" mono placeholder="{{items}}" {...bind(props, "list")} />
      <TextInput label="Show" mono templates={false} placeholder="Field" {...bind(props, "label")} />
      <MapInput
        label="Set"
        keyPlaceholder="Variable"
        valuePlaceholder="Field"
        value={outputs(step)}
        // Editing moves an older `name`/`value` into `set`.
        onChange={(set) => update((s) => setField(setField(setField(s, "name", ""), "value", ""), "set", set))}
      />
    </Fields>
  );
}

export const pick = defineTool<"pick">({
  label: "Pick from list",
  icon: ListChecks,
  create: () => ({ tool: "pick" }),
  summary: (step) => Object.keys(outputs(step)).join(", "),
  dataflow: (step) => ({
    templates: [step.title ?? "", step.list ?? ""],
    sets: Object.keys(outputs(step)),
    problems: [
      ...(step.list?.trim() ? [] : ["No list"]),
      ...(Object.keys(outputs(step)).length > 0 ? [] : ["No variable to set"]),
    ],
  }),
  Editor: PickEditor,
});
