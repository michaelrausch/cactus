import { useState } from "react";
import { FileDown, Play, Plus, Search } from "lucide-react";
import { importCollection } from "@/actions/project";
import { runFolder } from "@/actions/request";
import { createScenarioIn } from "@/actions/scenario";
import { withShortcut } from "@/app/shortcuts";
import { IconButton } from "@/components/IconButton";
import { Logo } from "@/components/Logo";
import { useProjectStore } from "@/stores/projectStore";
import { EnvPicker } from "./EnvPicker";
import { ProjectSwitcher } from "./ProjectSwitcher";
import { ScenarioList } from "./ScenarioList";

function ScenarioToolbar({ filter, onFilterChange }: { filter: string; onFilterChange: (value: string) => void }) {
  return (
    <div className="flex items-center gap-1 px-2 pb-2">
      <div className="flex h-8 flex-1 items-center gap-2 rounded-lg bg-sidebar-accent/50 px-2.5 text-muted-foreground focus-within:bg-sidebar-accent">
        <Search className="size-3.5 shrink-0" />
        <input
          value={filter}
          onChange={(e) => onFilterChange(e.target.value)}
          placeholder="Search"
          className="min-w-0 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
        />
      </div>
      <IconButton label="Run all" tone="sidebar" onClick={() => void runFolder("")}>
        <Play />
      </IconButton>
      <IconButton
        label={withShortcut("New scenario", "newScenario")}
        tone="sidebar"
        onClick={() => createScenarioIn("")}
      >
        <Plus />
      </IconButton>
    </div>
  );
}

export function Sidebar() {
  const hasProject = useProjectStore((s) => s.project !== null);
  const [filter, setFilter] = useState("");

  return (
    <div className="flex h-full flex-col bg-sidebar">
      <div className="flex h-12 items-center gap-1 pr-2 pl-3">
        <Logo />
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
      <ScenarioList filter={filter} />
      {hasProject && <EnvPicker />}
    </div>
  );
}
