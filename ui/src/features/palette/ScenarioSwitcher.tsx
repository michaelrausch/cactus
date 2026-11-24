import { selectScenario } from "@/actions/scenario";
import { MethodBadge } from "@/components/MethodBadge";
import { CommandDialog, CommandEmpty, CommandInput, CommandItem, CommandList } from "@/components/ui/command";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

/** Command palette for jumping to any scenario by title or path. */
export function ScenarioSwitcher() {
  const entries = useProjectStore((s) => s.entries);
  const open = useUiStore((s) => s.switcherOpen);
  const setOpen = useUiStore((s) => s.setSwitcherOpen);

  return (
    <CommandDialog open={open} onOpenChange={setOpen}>
      <CommandInput placeholder="Jump to scenario…" />
      <CommandList>
        <CommandEmpty>No match.</CommandEmpty>
        {entries.map((entry) => (
          <CommandItem
            key={entry.relPath}
            value={`${entry.title} ${entry.relPath}`}
            onSelect={() => {
              setOpen(false);
              void selectScenario(entry.relPath);
            }}
          >
            <MethodBadge method={entry.method} className="w-12 text-[10px]" />
            <span className="flex-1 truncate">{entry.title}</span>
            <span className="truncate text-xs text-muted-foreground">{entry.relPath}</span>
          </CommandItem>
        ))}
      </CommandList>
    </CommandDialog>
  );
}
