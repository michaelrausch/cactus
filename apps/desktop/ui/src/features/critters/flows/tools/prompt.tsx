import { MessageSquareText, Plus, X } from "lucide-react";
import type { PromptVar } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { setField } from "@/domain/flowEdit";
import { bind, Fields, TextInput } from "./fields";
import { defineTool, type ToolEditorProps } from "./types";

const names = (vars: readonly PromptVar[] | undefined): string[] =>
  (vars ?? []).map((v) => v.name?.trim() ?? "").filter(Boolean);

/** One value to ask for: name, label, default, and whether it's typed hidden. */
function VarRow({
  value,
  onChange,
  onRemove,
}: {
  value: PromptVar;
  onChange: (value: PromptVar) => void;
  onRemove: () => void;
}) {
  const field = <K extends keyof PromptVar>(key: K) => ({
    value: (value[key] as string | undefined) ?? "",
    onChange: (event: { target: { value: string } }) =>
      onChange(setField(value, key, event.target.value as PromptVar[K])),
  });
  return (
    <div className="flex flex-col gap-1.5 rounded-md border p-2">
      <div className="flex items-center gap-1.5">
        <Input aria-label="Name" placeholder="name" className="h-7 flex-1 font-mono text-xs" {...field("name")} />
        <label className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <input
            type="checkbox"
            className="size-3.5 accent-brand-2"
            checked={value.secret ?? false}
            onChange={(event) => onChange(setField(value, "secret", event.target.checked))}
          />
          Secret
        </label>
        <button
          type="button"
          aria-label={`Remove ${value.name || "variable"}`}
          onClick={onRemove}
          className="grid size-6 place-items-center rounded text-muted-foreground focus-ring hover:text-foreground"
        >
          <X className="size-3.5" />
        </button>
      </div>
      <div className="flex gap-1.5">
        <Input aria-label="Label" placeholder="Label" className="h-7 flex-1 text-xs" {...field("label")} />
        <Input
          aria-label="Default"
          placeholder="Default"
          type={value.secret ? "password" : "text"}
          className="h-7 flex-1 text-xs"
          {...field("default")}
        />
      </div>
    </div>
  );
}

function PromptEditor(props: ToolEditorProps<"prompt">) {
  const { step, update } = props;
  const vars = step.vars ?? [];
  const setVars = (next: PromptVar[]) => update((s) => setField(s, "vars", next));
  return (
    <Fields>
      <TextInput label="Title" {...bind(props, "title")} />
      <div className="flex flex-col gap-1.5">
        <span className="text-xs text-muted-foreground">Variables</span>
        {vars.map((value, i) => (
          <VarRow
            // Keyed by position: keying by name would remount the row on every keystroke.
            // oxlint-disable-next-line react/no-array-index-key
            key={i}
            value={value}
            onChange={(next) => setVars(vars.map((v, j) => (j === i ? next : v)))}
            onRemove={() => setVars(vars.filter((_, j) => j !== i))}
          />
        ))}
        <Button size="sm" variant="ghost" className="self-start" onClick={() => setVars([...vars, {}])}>
          <Plus /> Variable
        </Button>
      </div>
    </Fields>
  );
}

export const prompt = defineTool<"prompt">({
  label: "Prompt",
  icon: MessageSquareText,
  create: () => ({ tool: "prompt", vars: [{}] }),
  summary: (step) => names(step.vars).join(", "),
  dataflow: (step) => ({
    templates: [step.title ?? "", ...(step.vars ?? []).flatMap((v) => [v.label ?? "", v.default ?? ""])],
    sets: names(step.vars),
    problems: (step.vars ?? []).some((v) => !v.name?.trim()) ? ["A variable has no name"] : [],
  }),
  Editor: PromptEditor,
});
