import { Fragment } from "react";
import { FolderOpen } from "lucide-react";
import { openProject } from "@/actions/project";
import { SHORTCUTS, WELCOME_SHORTCUTS } from "@/app/shortcuts";
import { ShortcutKeys } from "@/components/Kbd";
import { Logo } from "@/components/Logo";
import { useProjectStore } from "@/stores/projectStore";
import { shortcutLabel } from "@/app/shortcuts";

/** Shown when no scenario is open. */
export function WelcomeScreen() {
  const hasProject = useProjectStore((s) => s.project !== null);
  return (
    <div className="grid h-full place-items-center">
      <div className="flex flex-col items-center gap-4 text-center">
        <Logo className="size-12" />
        <p className="mt-2 text-sm text-muted-foreground">
          {hasProject
            ? `Pick a request, or press ${shortcutLabel("newScenario")} to make one.`
            : "Requests as plain YAML files. Fast."}
        </p>
        {!hasProject && (
          <button
            type="button"
            onClick={() => void openProject()}
            className="flex items-center gap-2 rounded-full bg-brand-2 px-4 py-2 text-sm font-medium text-brand-foreground shadow-lg shadow-brand-2/20 focus-ring hover:brightness-110"
          >
            <FolderOpen className="size-4" /> Open a folder
          </button>
        )}
        <div className="mt-2 grid grid-cols-[auto_auto] gap-x-4 gap-y-1 text-xs text-muted-foreground">
          {WELCOME_SHORTCUTS.map((id) => (
            <Fragment key={id}>
              <ShortcutKeys id={id} />
              <span className="text-left">{SHORTCUTS[id].description}</span>
            </Fragment>
          ))}
        </div>
      </div>
    </div>
  );
}
