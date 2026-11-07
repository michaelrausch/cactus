import { Check, ChevronsUpDown, GitBranch, Plus, Trash2 } from "lucide-react";
import type { GitStatus } from "@/api/client";
import { createBranch, deleteBranch, loadBranches, switchBranch } from "@/actions/git";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { timeAgo } from "@/lib/format";
import { useGitStore } from "@/stores/gitStore";

/** The checked-out branch; opens a menu to switch, create or delete branches. */
export function BranchMenu({ status }: { status: GitStatus }) {
  const branches = useGitStore((s) => s.branches);
  const others = branches.filter((branch) => !branch.current);
  const hasCommits = status.lastCommit !== null;

  return (
    <DropdownMenu onOpenChange={(open) => open && void loadBranches()}>
      <DropdownMenuTrigger className="-mx-2 flex min-w-0 items-center gap-2 rounded-md px-2 py-1 text-sm font-medium outline-none hover:bg-sidebar-accent">
        <GitBranch className="size-4 shrink-0 text-muted-foreground" />
        <span className="truncate">{status.branch ?? "detached HEAD"}</span>
        <ChevronsUpDown className="size-3.5 shrink-0 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-72">
        <DropdownMenuLabel className="text-xs text-muted-foreground">Switch branch</DropdownMenuLabel>
        {branches.map((branch) => (
          <DropdownMenuItem
            key={branch.name}
            disabled={branch.current}
            onSelect={() => switchBranch(branch.name)}
            className="gap-2"
          >
            <Check className={branch.current ? "size-4" : "invisible size-4"} />
            <span className="min-w-0 flex-1 truncate">{branch.name}</span>
            {branch.lastCommit && (
              <span className="shrink-0 text-xs text-muted-foreground">{timeAgo(branch.lastCommit.at)}</span>
            )}
          </DropdownMenuItem>
        ))}
        {!hasCommits && (
          <DropdownMenuItem disabled className="text-xs">
            Make a first commit to create branches
          </DropdownMenuItem>
        )}
        <DropdownMenuSeparator />
        <DropdownMenuItem disabled={!hasCommits} onSelect={createBranch}>
          <Plus /> New branch…
        </DropdownMenuItem>
        {others.length > 0 && (
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>
              <Trash2 /> Delete branch
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="w-56">
              {others.map((branch) => (
                <DropdownMenuItem key={branch.name} onSelect={() => deleteBranch(branch)} className="gap-2">
                  <span className="min-w-0 flex-1 truncate">{branch.name}</span>
                  {!branch.merged && <span className="shrink-0 text-xs text-warning">unmerged</span>}
                </DropdownMenuItem>
              ))}
            </DropdownMenuSubContent>
          </DropdownMenuSub>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
