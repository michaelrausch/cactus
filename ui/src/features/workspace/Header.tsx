import { Copy, PanelRight, Play } from "lucide-react";
import { runFolder } from "@/actions/request";
import { duplicateScenario } from "@/actions/scenario";
import { withShortcut } from "@/app/shortcuts";
import { IconButton } from "@/components/IconButton";
import { MethodBadge } from "@/components/MethodBadge";
import { baseName, dirOf } from "@/domain/paths";
import { displayTitle } from "@/domain/scenario";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";

const Separator = () => <span className="text-muted-foreground/50">/</span>;

function Breadcrumb({ project }: { project: string }) {
  const path = useEditorStore((s) => s.path);
  const scenario = useEditorStore((s) => s.scenario);
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
          <MethodBadge method={scenario.http.method} className="text-[11px]" />
          <span className="truncate font-medium">{displayTitle(scenario)}</span>
        </>
      )}
    </div>
  );
}

function EnvChip() {
  const activeEnv = useProjectStore((s) => s.activeEnv);
  return (
    <button
      type="button"
      onClick={() => useUiStore.getState().setEnvDialogOpen(true)}
      className="mr-1 flex h-6 items-center gap-1.5 rounded-full border px-2.5 text-xs text-muted-foreground hover:text-foreground"
      title={withShortcut("Environment", "variables")}
    >
      <span className={cn("size-1.5 rounded-full", activeEnv ? "bg-brand" : "bg-muted-foreground/50")} />
      {activeEnv ?? "no env"}
    </button>
  );
}

function ScenarioActions() {
  const path = useEditorStore((s) => s.path);
  if (!path) return null;
  const dir = dirOf(path);
  return (
    <>
      <IconButton label={`Run ${dir ? `${dir}/` : "all"}`} onClick={() => void runFolder(dir)}>
        <Play />
      </IconButton>
      <IconButton label={withShortcut("Duplicate", "duplicate")} onClick={() => void duplicateScenario(path)}>
        <Copy />
      </IconButton>
    </>
  );
}

export function Header() {
  const project = useProjectStore((s) => s.project);
  const responseVisible = useUiStore((s) => s.responseVisible);
  return (
    <header className="relative flex h-12 shrink-0 items-center gap-2 px-4">
      {project && (
        <>
          <Breadcrumb project={project} />
          <div className="ml-auto flex items-center gap-1">
            <EnvChip />
            <ScenarioActions />
            <IconButton
              label={withShortcut("Toggle response", "toggleResponse")}
              onClick={() => useUiStore.getState().toggleResponse()}
              active={responseVisible}
            >
              <PanelRight />
            </IconButton>
          </div>
        </>
      )}
    </header>
  );
}
