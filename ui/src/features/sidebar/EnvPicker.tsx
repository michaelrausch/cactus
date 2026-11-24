import { ChevronsUpDown, SlidersHorizontal } from "lucide-react";
import { selectEnv } from "@/actions/project";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { cn } from "@/lib/utils";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

const EnvDot = ({ active }: { active: boolean }) => (
  <span className={cn("size-2 shrink-0 rounded-full", active ? "bg-brand" : "bg-muted-foreground/50")} />
);

const openEnvDialog = () => useUiStore.getState().setEnvDialogOpen(true);

/** Footer control for choosing the active environment. */
export function EnvPicker() {
  const envs = useProjectStore((s) => s.envs);
  const activeEnv = useProjectStore((s) => s.activeEnv);

  return (
    <div className="flex items-center gap-1 border-t border-sidebar-border p-2">
      <DropdownMenu>
        <DropdownMenuTrigger className="flex h-8 min-w-0 flex-1 items-center gap-2 rounded-lg px-2 text-left text-sm focus-ring hover:bg-sidebar-accent">
          <EnvDot active={!!activeEnv} />
          <span className="min-w-0 flex-1 truncate">{activeEnv ?? "No environment"}</span>
          <ChevronsUpDown className="size-3.5 text-muted-foreground" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" side="top" className="w-56">
          <DropdownMenuItem onSelect={() => void selectEnv(null)}>
            <EnvDot active={false} /> No environment
          </DropdownMenuItem>
          {envs.map((name) => (
            <DropdownMenuItem key={name} onSelect={() => void selectEnv(name)}>
              <EnvDot active /> {name}
            </DropdownMenuItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={openEnvDialog}>
            <SlidersHorizontal /> Edit variables…
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}
