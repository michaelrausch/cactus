import { shortcutKeys, type ShortcutId } from "@/app/shortcuts";

export function ShortcutKeys({ id, className }: { id: ShortcutId; className?: string }) {
  return <kbd className={className}>{shortcutKeys(id).join(" ")}</kbd>;
}
