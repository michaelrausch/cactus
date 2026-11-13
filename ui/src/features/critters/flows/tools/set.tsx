import { Variable } from "lucide-react";
import { generatorSuggester } from "@/domain/generators";
import { bind, Fields, MapInput } from "./fields";
import { defineTool } from "./types";

export const set = defineTool<"set">({
  label: "Set values",
  icon: Variable,
  create: () => ({ tool: "set" }),
  summary: (step) => Object.keys(step.values ?? {}).join(", "),
  dataflow: (step) => ({ templates: Object.values(step.values ?? {}), sets: Object.keys(step.values ?? {}) }),
  Editor: (props) => (
    <Fields>
      <MapInput
        label="Values"
        keyPlaceholder="Variable"
        valuePlaceholder="$uuid"
        suggest={generatorSuggester}
        {...bind(props, "values")}
      />
    </Fields>
  ),
});
