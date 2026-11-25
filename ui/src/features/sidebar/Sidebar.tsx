import { useState } from "react";
import { FileDown, FolderPlus, Play, Plus, Search } from "lucide-react";
import { importCollection } from "@/actions/project";
import { runFolder } from "@/actions/request";
import { createFolder, createScenarioIn } from "@/actions/scenario";
import { withShortcut } from "@/app/shortcuts";
import { IconButton } from "@/components/IconButton";
import { Logo } from "@/components/Logo";
import { isMac } from "@/lib/platform";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { CritterPicker } from "./CritterPicker";
import { CritterTree } from "./CritterTree";
import { EnvPicker } from "./EnvPicker";
import { ProjectSwitcher } from "./ProjectSwitcher";
import { ScenarioList } from "./ScenarioList";

function ScenarioToolbar({ filter, onFilterChange }: { filter: string; onFilterChange: (value: string) => void }) {
  return (
    <div className="flex items-center gap-1 px-2 pb-2">
      <div className="flex h-8 min-w-0 flex-1 items-center gap-2 rounded-lg bg-sidebar-accent/50 px-2.5 text-muted-foreground focus-within:bg-sidebar-accent focus-within:ring-1 focus-within:ring-ring">
        <Search className="size-3.5 shrink-0" />
        <input
          value={filter}
          onChange={(e) => onFilterChange(e.target.value)}
          placeholder="Search"
          aria-label="Search requests"
          className="min-w-0 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
        />
      </div>
      <IconButton label="Run all" tone="sidebar" onClick={() => void runFolder("")}>
        <Play />
      </IconButton>
      <IconButton label="New folder" tone="sidebar" onClick={() => createFolder()}>
        <FolderPlus />
      </IconButton>
      <IconButton
        label={withShortcut("New request", "newScenario")}
        tone="sidebar"
        onClick={() => createScenarioIn("")}
      >
        <Plus />
      </IconButton>
    </div>
  );
}

/** The cactus: opens Settings. */
function SettingsButton() {
  const openSettings = useUiStore((s) => s.setSettingsOpen);
  const label = withShortcut("Settings", "settings");
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={() => openSettings(true)}
      className="group -ml-1 grid size-7 shrink-0 place-items-center rounded-md focus-ring transition hover:bg-sidebar-accent"
    >
      <Logo className="transition-transform duration-300 group-hover:scale-110 group-hover:-rotate-6 group-active:scale-95" />
    </button>
  );
}

/** The project switcher, env and critter pickers, request list and critter tree. */
export function Sidebar() {
  const project = useProjectStore((s) => s.project);
  const hasProject = project !== null;
  const [filter, setFilter] = useState("");

  return (
    <div className="flex h-full flex-col bg-sidebar">
      {/* On macOS, a drag strip that holds the traffic lights. */}
      {isMac && <div data-tauri-drag-region className="h-8 shrink-0" />}
      <div data-tauri-drag-region className="flex h-12 items-center gap-1 pr-2 pl-3">
        <SettingsButton />
        <div className="min-w-0 flex-1">
          <ProjectSwitcher />
        </div>
        {hasProject && (
          <IconButton label="Import Postman / OpenAPI" tone="sidebar" onClick={() => void importCollection()}>
            <FileDown />
          </IconButton>
        )}
      </div>
      {hasProject && <ScenarioToolbar filter={filter} onFilterChange={setFilter} />}
      {/* Keyed so another project starts scrolled to the top. */}
      <div key={project ?? ""} className="min-h-0 flex-1 overflow-y-auto px-2 pb-2">
        <ScenarioList filter={filter} />
        {hasProject && <CritterTree filter={filter} />}
      </div>
      {hasProject && (
        <div className="flex items-center gap-1 border-t border-sidebar-border p-2">
          <EnvPicker />
          <CritterPicker />
        </div>
      )}
    </div>
  );
}
