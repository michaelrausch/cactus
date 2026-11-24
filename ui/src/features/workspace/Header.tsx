import { Copy, PanelRight, Play, Save } from "lucide-react";
import { runFolder } from "@/actions/request";
import { duplicateScenario } from "@/actions/scenario";
import { saveScenario } from "@/actions/saving";
import { withShortcut } from "@/app/shortcuts";
import { IconButton } from "@/components/IconButton";
import { MethodBadge } from "@/components/MethodBadge";
import { baseName, dirOf } from "@/domain/paths";
import { displayTitle } from "@/domain/scenario";
import { isDirty, useEditorStore } from "@/stores/editorStore";
import { useProjectStore } from "@/stores/projectStore";
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
          <MethodBadge method={scenario.http.method} className="text-[11px]" />
          <span className="truncate font-medium">{displayTitle(scenario)}</span>
          {dirty && <span title="Unsaved changes" className="size-2 shrink-0 rounded-full bg-brand" />}
        </>
      )}
    </div>
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
