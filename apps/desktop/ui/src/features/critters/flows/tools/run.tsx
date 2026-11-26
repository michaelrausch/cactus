import { Workflow } from "lucide-react";
import { useCritterStore } from "@/stores/critterStore";
import { useFlowStore } from "@/stores/flowStore";
import { bind, ChoiceInput, Fields } from "./fields";
import { defineTool, type ToolEditorProps } from "./types";

function RunEditor(props: ToolEditorProps<"run">) {
  const critters = useCritterStore((s) => s.critters);
  const own = useFlowStore((s) => s.open?.critter);
  const whose = props.step.critter || own;
  const flows = critters.find((c) => c.id === whose)?.flows ?? [];
  return (
    <Fields>
      <ChoiceInput
        label="Critter"
        none="This critter"
        options={critters.filter((c) => c.id !== own).map((c) => ({ value: c.id, label: c.name }))}
        {...bind(props, "critter")}
      />
      <ChoiceInput
        label="Flow"
        none="Choose…"
        options={flows.map((f) => ({ value: f.id, label: f.name }))}
        {...bind(props, "flow")}
      />
    </Fields>
  );
}

export const run = defineTool<"run">({
  label: "Run flow",
  icon: Workflow,
  create: () => ({ tool: "run" }),
  summary: (step) => [step.critter, step.flow].filter(Boolean).join("/"),
  dataflow: (step) => ({ problems: step.flow?.trim() ? [] : ["No flow"] }),
  Editor: RunEditor,
});
