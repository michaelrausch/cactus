import { ChevronsUpDown, FolderOpen } from "lucide-react";
import { openProject } from "@/actions/project";
import { shortcutKeys } from "@/app/shortcuts";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { baseName } from "@/domain/paths";
import { useProjectStore } from "@/stores/projectStore";

/** Shows the open project; opens recent projects or a new folder. */
export function ProjectSwitcher() {
  const project = useProjectStore((s) => s.project);
  const recent = useProjectStore((s) => s.recentProjects);
  const name = project ? baseName(project) : null;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm outline-none hover:bg-sidebar-accent">
        <span className="min-w-0 flex-1 truncate text-base font-semibold">{name ?? "Open a project"}</span>
        <ChevronsUpDown className="size-3.5 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-60">
        {recent.length > 0 && (
          <>
            <DropdownMenuLabel className="text-xs text-muted-foreground">Recent</DropdownMenuLabel>
            {recent.map((path) => (
              <DropdownMenuItem key={path} onSelect={() => void openProject(path)} title={path}>
                <span className="truncate">{baseName(path)}</span>
              </DropdownMenuItem>
            ))}
            <DropdownMenuSeparator />
          </>
        )}
        <DropdownMenuItem onSelect={() => void openProject()}>
          <FolderOpen /> Open folder…{" "}
          <span className="ml-auto text-xs text-muted-foreground">{shortcutKeys("openProject").join(" ")}</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
