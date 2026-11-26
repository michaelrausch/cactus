import { Field } from "@/components/Field";
import { TemplateField } from "@/components/template/TemplateField";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

/** Looks like `Input`; the border is on the box so the text inside lines up with its colours. */
const FIELD_BOX =
  "rounded-lg border border-input bg-input/30 transition-colors focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50";
/** `Input`'s height less its border. */
const FIELD_TEXT = "h-[calc(2rem-2px)] px-2.5 py-1 font-mono text-xs md:text-sm";

/** A labelled field that takes `{{variables}}`. */
export function TextField({
  label,
  value,
  placeholder,
  onChange,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
}) {
  return (
    <Field label={label}>
      <TemplateField
        className={FIELD_BOX}
        fieldClassName={FIELD_TEXT}
        value={value}
        placeholder={placeholder}
        aria-label={label}
        onChange={onChange}
      />
    </Field>
  );
}

/** A labelled select over `options`, value → label. */
export function ChoiceField<T extends string>({
  label,
  value,
  options,
  onChange,
  className = "w-28",
}: {
  label: string;
  value: T;
  options: Record<T, string>;
  onChange: (value: T) => void;
  className?: string;
}) {
  const values = Object.keys(options) as T[];
  return (
    <Field label={label}>
      <Select value={value} onValueChange={(next) => onChange(values.find((v) => v === next) ?? value)}>
        <SelectTrigger className={className}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {values.map((option) => (
            <SelectItem key={option} value={option}>
              {options[option]}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
}
