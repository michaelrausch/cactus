import { useMemo } from "react";
import { ShieldCheck } from "lucide-react";
import type { Assertion } from "@/api/client";
import { setField } from "@/domain/flowEdit";
import { itemFields, listVariable } from "@/domain/itemFields";
import { wordSuggester } from "@/domain/suggestions";
import { useFlowStore } from "@/stores/flowStore";
import { bind, ChecksInput, Fields, TextInput } from "./fields";
import { defineTool, type ToolEditorProps } from "./types";

const describe = (checks: readonly Assertion[] | undefined): string[] =>
  (checks ?? []).filter((c) => !c.disabled).map((c) => `${c.expression} ${c.operator} ${c.value ?? ""}`.trim());

/** The list's last value in this run, if it named one variable: what a request captured or a tool set. */
function useLastValue(list: string | undefined): string | undefined {
  const name = listVariable(list);
  return useFlowStore((s) => {
    if (!name) return undefined;
    for (const result of s.run.results.toReversed()) {
      const value = result?.response?.captured[name] ?? result?.output?.values?.[name];
      if (value !== undefined) return value;
    }
    return undefined;
  });
}

function AssertEditor(props: ToolEditorProps<"assert">) {
  const { step, update } = props;
  const each = step.each ?? {};
  // An item's fields, once a run has shown what the list holds.
  const last = useLastValue(each.list);
  const fields = useMemo(() => wordSuggester(itemFields(last)), [last]);
  const setEach = <K extends keyof typeof each>(key: K, value: (typeof each)[K]) =>
    update((s) => setField(s, "each", setField(s.each ?? {}, key, value)));
  return (
    <Fields>
      <ChecksInput label="Checks" keyPlaceholder="{{variable}}.length" templates {...bind(props, "checks")} />
      <TextInput
        label="Each item of"
        mono
        placeholder="{{list}}"
        value={each.list}
        onChange={(v) => setEach("list", v)}
      />
      <ChecksInput
        label="Each item"
        keyPlaceholder="Field"
        suggest={fields}
        value={each.checks}
        onChange={(checks) => setEach("checks", checks)}
      />
    </Fields>
  );
}

export const assert = defineTool<"assert">({
  label: "Assert",
  icon: ShieldCheck,
  create: () => ({ tool: "assert" }),
  summary: (step) => {
    const each = step.each?.list ? [`each of ${step.each.list}`] : [];
    return [...describe(step.checks), ...each].join(", ");
  },
  dataflow: (step) => {
    const checks = [...(step.checks ?? []), ...(step.each?.checks ?? [])];
    const hasEach = Boolean(step.each?.list?.trim());
    return {
      templates: [
        ...(step.checks ?? []).map((c) => c.expression),
        step.each?.list ?? "",
        ...checks.map((c) => c.value ?? ""),
      ],
      problems: [
        ...((step.checks ?? []).length === 0 && !hasEach ? ["No checks"] : []),
        ...(hasEach && (step.each?.checks ?? []).length === 0 ? ["No checks for each item"] : []),
        ...((step.each?.checks ?? []).length > 0 && !hasEach ? ["No list for each item"] : []),
      ],
    };
  },
  Editor: AssertEditor,
});
