import { useState } from "react";
import { FolderOpen } from "lucide-react";
import { openProject } from "@/actions/project";
import { unlock, wipeSecrets } from "@/actions/vault";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";
import { baseName } from "@/domain/paths";
import { useProjectStore } from "@/stores/projectStore";
import { CodeBoxes } from "./CodeBoxes";
import { ErrorLine, PasswordInput, StackedForm, useRunner } from "./PasswordFields";

type View = "unlock" | "wipe" | "projects";

/** Covers the whole app while the open project is locked: unlock it, wipe its secrets or open another. */
export function LockScreen() {
  const project = useProjectStore((s) => s.project);
  return (
    <div className="flex h-full flex-col">
      {/* The window's title bar is hidden behind the app, so it needs its own drag strip. */}
      <div data-tauri-drag-region className="h-10 shrink-0" />
      <div className="flex min-h-0 flex-1 items-center justify-center px-8 pb-10">
        {/* Keyed so opening another locked project starts back at its password. */}
        <Locked key={project} project={project} />
      </div>
    </div>
  );
}

function Locked({ project }: { project: string | null }) {
  const [view, setView] = useState<View>("unlock");
  const back = () => setView("unlock");
  const name = project ? baseName(project) : "Locked";
  return (
    <div className="flex max-h-full w-full max-w-xs flex-col items-center gap-5">
      <Logo className="size-16 shrink-0 drop-shadow-[0_0_18px_color-mix(in_oklch,var(--color-brand-2)_40%,transparent)]" />
      <h1 className="text-xl font-semibold tracking-tight">{view === "projects" ? "Change project" : name}</h1>
      {view === "unlock" && <UnlockForm forgot={() => setView("wipe")} change={() => setView("projects")} />}
      {view === "wipe" && <WipeForm back={back} />}
      {view === "projects" && <ProjectList current={project} back={back} />}
    </div>
  );
}

/** The password, then the authenticator code if the project asks for one. */
function UnlockForm({ forgot, change }: { forgot: () => void; change: () => void }) {
  const [password, setPassword] = useState("");
  const [code, setCode] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const { run, busy, error } = useRunner();
  /** `entered` is the code as just completed, before the state catches up. */
  const submit = (entered: string | null = code) =>
    run(async () => {
      const result = await unlock(password, entered);
      if (result.codeRequired) setCode("");
      // Remount the boxes after a wrong code, so the first one has focus again.
      if (result.error && entered !== null) setAttempt((n) => n + 1);
      return result.error;
    });
  const ready = !busy && Boolean(password) && (code === null || code.length === 6);
  return (
    <StackedForm onSubmit={() => ready && submit()}>
      {code === null ? (
        <PasswordInput label="Password" value={password} onChange={setPassword} autoFocus />
      ) : (
        <CodeBoxes key={attempt} value={code} onChange={setCode} onComplete={submit} disabled={busy} />
      )}
      <ErrorLine error={error} />
      <Button type="submit" className="mt-1" disabled={!ready}>
        Unlock
      </Button>
      <div className="flex justify-between">
        <Button type="button" variant="link" size="sm" className="px-0 text-muted-foreground" onClick={change}>
          Change project
        </Button>
        <Button type="button" variant="link" size="sm" className="px-0 text-muted-foreground" onClick={forgot}>
          Forgot password?
        </Button>
      </div>
    </StackedForm>
  );
}

/** Recent projects other than this one, and a folder picker. */
function ProjectList({ current, back }: { current: string | null; back: () => void }) {
  const recent = useProjectStore((s) => s.recentProjects).filter((path) => path !== current);
  return (
    <div className="flex min-h-0 w-full flex-col gap-3">
      {recent.length > 0 && (
        <ul className="min-h-0 divide-y overflow-y-auto rounded-xl border bg-card/60">
          {recent.map((path) => (
            <li key={path}>
              <button
                type="button"
                title={path}
                onClick={() => void openProject(path)}
                className="flex w-full flex-col px-4 py-2.5 text-left focus-ring hover:bg-accent/50"
              >
                <span className="truncate text-sm font-medium">{baseName(path)}</span>
                <span className="truncate text-xs text-muted-foreground">{path}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
      <Button variant="outline" className="shrink-0" onClick={() => void openProject()}>
        <FolderOpen /> Open folder…
      </Button>
      <Button variant="ghost" className="shrink-0" onClick={back}>
        Back
      </Button>
    </div>
  );
}

function WipeForm({ back }: { back: () => void }) {
  const { run, busy, error } = useRunner();
  return (
    <div className="flex w-full flex-col gap-3 text-center">
      <p className="text-sm">Wipe all secrets and captured values?</p>
      <ErrorLine error={error} />
      <div className="flex justify-center gap-2">
        <Button variant="ghost" onClick={back}>
          Back
        </Button>
        <Button variant="destructive" disabled={busy} onClick={() => run(wipeSecrets)}>
          Wipe
        </Button>
      </div>
    </div>
  );
}
