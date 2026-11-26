import { GitBranch, PanelRight, Play, Save, TriangleAlert } from "lucide-react";
import { PRO } from "@edition/flag";
import { openChanges } from "@/actions/git";
import { runFolder } from "@/actions/request";
import { saveScenario } from "@/actions/saving";
import { withShortcut } from "@/app/shortcuts";
import { IconButton } from "@/components/IconButton";
import { MethodBadge } from "@/components/MethodBadge";
import { WindowControls } from "@/components/WindowControls";
import { baseName, dirOf } from "@/domain/paths";
import { stagedFiles, unstagedFiles } from "@/domain/git";
import { displayTitle } from "@/domain/scenario";
import { isMac } from "@/lib/platform";
import { cn } from "@/lib/utils";
import { isDirty, useEditorStore } from "@/stores/editorStore";
import { useGitStore } from "@/stores/gitStore";
import { useProjectStore } from "@/stores/projectStore";
import { hasResponseContent, useResponseStore } from "@/stores/responseStore";
import { useUiStore } from "@/stores/uiStore";

const Separator = () => <span className="text-muted-foreground/50">/</span>;

function Breadcrumb({ project }: { project: string }) {
  const path = useEditorStore((s) => s.path);
  const scenario = useEditorStore((s) => s.scenario);
  const dirty = useEditorStore((s) => (s.path ? isDirty(s.buffers[s.path]) : false));
  const dir = path ? dirOf(path) : "";
  return (
    <div className="flex min-w-0 flex-1 items-center gap-2 text-sm">
      <span className="truncate text-muted-foreground">{baseName(project)}</span>
      {dir && (
        <>
          <Separator />
          <span className="truncate text-muted-foreground">{dir}</span>
        </>
      )}
      {scenario && (
        <>
          <Separator />
          <MethodBadge method={scenario.http.method} className="text-xs" />
          <span className="truncate font-medium">{displayTitle(scenario)}</span>
          {dirty && (
            <span title="Unsaved changes" className="size-2 shrink-0 rounded-full bg-brand">
              <span className="sr-only">Unsaved changes</span>
            </span>
          )}
        </>
      )}
    </div>
  );
}

const CHIP =
  "mr-1 flex h-6 max-w-56 items-center gap-1.5 rounded-full border px-2.5 text-xs text-muted-foreground hover:text-foreground";

/**
 * The project's git state, opening the Changes dialog: branch plus staged and
 * unstaged counts, an offer to set up git, or why the repository can't be read.
 */
function GitChip() {
  const status = useGitStore((s) => s.status);
  const noRepo = useGitStore((s) => s.noRepo);
  const error = useGitStore((s) => s.error);

  if (error) {
    return (
      <span className={cn(CHIP, "hover:text-muted-foreground")} title={error}>
        <TriangleAlert className="size-3.5 shrink-0 text-warning" />
        git unavailable
      </span>
    );
  }
  if (noRepo) {
    return (
      <button
        type="button"
        onClick={openChanges}
        className={cn(CHIP, "border-dashed")}
        title="Track this collection with git"
      >
        <GitBranch className="size-3.5 shrink-0" />
        Set up git
      </button>
    );
  }
  if (!status) return null;
  const staged = stagedFiles(status).length;
  const unstaged = unstagedFiles(status).length;
  const counts = [staged && `${staged} staged`, unstaged && `${unstaged} changed`].filter(Boolean).join(", ");
  return (
    <button
      type="button"
      onClick={openChanges}
      className={CHIP}
      title={withShortcut(counts ? `Changes: ${counts}` : "Changes: none", "changes")}
    >
      <GitBranch className="size-3.5 shrink-0" />
      <span className="truncate">{status.branch ?? "detached"}</span>
      {staged > 0 && <span className="shrink-0 text-success">+{staged}</span>}
      {unstaged > 0 && <span className="shrink-0 text-warning">~{unstaged}</span>}
    </button>
  );
}

function ScenarioActions() {
  const path = useEditorStore((s) => s.path);
  const dirty = useEditorStore((s) => (s.path ? isDirty(s.buffers[s.path]) : false));
  if (!path) return null;
  const dir = dirOf(path);
  return (
    <>
      <IconButton label={withShortcut("Save", "save")} onClick={() => void saveScenario(path)} active={dirty}>
        <Save />
      </IconButton>
      <IconButton label={`Run ${dir ? `${dir}/` : "all"}`} onClick={() => void runFolder(dir)}>
        <Play />
      </IconButton>
    </>
  );
}

/** The window's top bar, and its drag handle. */
export function Header() {
  const project = useProjectStore((s) => s.project);
  const responseVisible = useUiStore((s) => s.responseVisible);
  const hasResponse = useResponseStore(hasResponseContent);
  // A critter or flow has its own header below.
  const editing = useUiStore((s) => s.view === "editor");
  return (
    <header data-tauri-drag-region className="relative flex h-12 shrink-0 items-center gap-2 px-4">
      {project && (
        <>
          {editing ? (
            <Breadcrumb project={project} />
          ) : (
            <span className="min-w-0 flex-1 truncate text-sm text-muted-foreground">{baseName(project)}</span>
          )}
          <div className="ml-auto flex items-center gap-1">
            <GitChip />
            {editing && <ScenarioActions />}
            {editing && hasResponse && (
              <IconButton
                label={withShortcut("Toggle response", "toggleResponse")}
                onClick={() => useUiStore.getState().toggleResponse()}
                active={responseVisible}
              >
                <PanelRight />
              </IconButton>
            )}
          </div>
        </>
      )}
      {PRO && (
        <span
          className={cn(
            "shrink-0 rounded-sm px-1.5 py-0.5 text-2xs font-semibold tracking-wider text-brand-2 ring-1 ring-brand-2/50",
            !project && "ml-auto",
          )}
        >
          PRO
        </span>
      )}
      {!isMac && <WindowControls />}
    </header>
  );
}
