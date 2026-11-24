import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { AUTO_LOCK_OPTIONS, autoLockLabel } from "@/domain/password";

const NEVER = "never";
const toValue = (minutes: number | null): string => (minutes === null ? NEVER : String(minutes));
const fromValue = (value: string): number | null => (value === NEVER ? null : Number(value));

export function AutoLockSelect({
  value,
  onChange,
  id,
}: {
  value: number | null;
  onChange: (minutes: number | null) => void;
  id?: string;
}) {
  return (
    <Select value={toValue(value)} onValueChange={(next) => onChange(fromValue(next))}>
      <SelectTrigger id={id} className="h-8 w-36">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {AUTO_LOCK_OPTIONS.map((minutes) => (
          <SelectItem key={toValue(minutes)} value={toValue(minutes)}>
            {autoLockLabel(minutes)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
