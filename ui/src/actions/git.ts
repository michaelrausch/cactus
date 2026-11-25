/**
 * Git workflows for the open project: status, staging, diffs and commits.
 * Every change refreshes the status, which also keeps the diff pane current.
 */
import { toast } from "sonner";
import { backend, type BranchInfo, type FileStatus, type GitIdentity, type ProjectPath } from "@/api/client";
import { isUntracked, nextSelection, sameSelection, unstagePaths, type DiffSelection } from "@/domain/git";
import { errorMessage, reportErrors, warnQuietly } from "@/lib/errors";
import { dirtyPaths, useEditorStore } from "@/stores/editorStore";
import { useGitStore } from "@/stores/gitStore";
import { useProjectStore } from "@/stores/projectStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useUiStore } from "@/stores/uiStore";
import { refreshEntries, refreshEnvs } from "./project";
import { hasUnsavedChanges, saveAll } from "./saving";
import { selectScenario } from "./scenario";

const currentProject = (): string | null => useProjectStore.getState().project;

/** Only the newest status request's answer is kept. */
let latestStatusRequest = 0;

/** Reload the project's git status, then the diff on show. */
export async function refreshGitStatus(): Promise<void> {
  const project = currentProject();
  const request = ++latestStatusRequest;
  if (!project) {
    useGitStore.getState().reset();
    return;
  }
  try {
    const status = await backend.gitStatus(project);
    if (request !== latestStatusRequest) return;
    const git = useGitStore.getState();
    git.statusLoaded(status);
    const selection = status ? nextSelection(status, git.selected) : null;
    if (!sameSelection(selection, git.selected)) git.selectionChanged(selection);
    await loadDiff(selection);
  } catch (error) {
    if (request === latestStatusRequest) useGitStore.getState().statusFailed(errorMessage(error));
  }
}

export async function showDiff(selection: DiffSelection): Promise<void> {
  useGitStore.getState().selectionChanged(selection);
  await loadDiff(selection);
}

async function loadDiff(selection: DiffSelection | null): Promise<void> {
  const project = currentProject();
  const isCurrent = () => sameSelection(useGitStore.getState().selected, selection);
  if (!project || !selection) {
    useGitStore.getState().diffLoaded(null);
    return;
  }
  const diff = await warnQuietly(() => backend.gitDiff(project, selection.path, selection.side), "Couldn't load diff");
  if (isCurrent()) useGitStore.getState().diffLoaded(diff ?? null);
}

/** Open the Changes dialog, or offer to set up a repository when there is none. */
export function openChanges(): void {
  const { status, noRepo } = useGitStore.getState();
  if (noRepo) {
    initRepository();
    return;
  }
  if (!status) return;
  useUiStore.getState().setGitDialogOpen(true);
  void refreshGitStatus();
}

export function initRepository(): void {
  useUiStore.getState().askConfirm({
    title: "Set up git?",
    description:
      "This makes the collection folder a git repository, so its changes can be reviewed and committed here. Local state in .cactus/ stays out of it.",
    action: "Set up git",
    onConfirm: async () => {
      const project = currentProject();
      if (!project) return;
      const done = await reportErrors(async () => {
        await backend.gitInit(project);
        return true;
      }, "Couldn't set up git");
      await refreshGitStatus();
      if (done) useUiStore.getState().setGitDialogOpen(true);
    },
  });
}

/** Close the dialog and show the request in the editor. */
export async function openInEditor(path: ProjectPath): Promise<void> {
  if (!useProjectStore.getState().entries.some((entry) => entry.relPath === path)) return;
  useUiStore.getState().setGitDialogOpen(false);
  await selectScenario(path);
}

async function changeIndex(work: (project: string) => Promise<unknown>, context: string): Promise<void> {
  const project = currentProject();
  if (!project) return;
  await reportErrors(() => work(project), context);
  await refreshGitStatus();
}

/**
 * Staging takes the files as they are on disk, so offer to save any of
 * `paths` with unsaved edits first.
 */
function stageAfterSaving(paths: ProjectPath[], proceed: () => Promise<void>): void {
  if (paths.length === 0) {
    void proceed();
    return;
  }
  useUiStore.getState().askConfirm({
    title: "Unsaved changes",
    description: "Some requests have unsaved edits, which staging won't include unless they are saved.",
    action: "Save and stage",
    onConfirm: async () => {
      if (await saveAll(paths)) await proceed();
    },
    alternative: { action: "Stage without them", onConfirm: proceed },
  });
}

const unsavedAmong = (paths: ProjectPath[] | null): ProjectPath[] => {
  const dirty = dirtyPaths(useEditorStore.getState().buffers);
  return paths ? dirty.filter((path) => paths.includes(path)) : dirty;
};

export function stage(paths: ProjectPath[]): void {
  stageAfterSaving(unsavedAmong(paths), () =>
    changeIndex((project) => backend.gitStage(project, paths), "Stage failed"),
  );
}

export function stageAll(): void {
  stageAfterSaving(unsavedAmong(null), () => changeIndex((project) => backend.gitStageAll(project), "Stage failed"));
}

export const unstage = (file: FileStatus): Promise<void> =>
  changeIndex((project) => backend.gitUnstage(project, unstagePaths(file)), "Unstage failed");

export const unstageAll = (): Promise<void> =>
  changeIndex((project) => backend.gitUnstageAll(project), "Unstage failed");

/** Throw away a file's unstaged changes, after asking. Open editors follow via the file watcher. */
export function discard(file: FileStatus): void {
  const untracked = isUntracked(file);
  useUiStore.getState().askConfirm({
    title: untracked ? "Delete file?" : "Discard changes?",
    destructive: true,
    description: untracked
      ? `${file.path} isn't tracked by git, so discarding it deletes the file.`
      : `The unstaged changes to ${file.path} will be lost. Staged changes are kept.`,
    action: untracked ? "Delete" : "Discard",
    onConfirm: () => changeIndex((project) => backend.gitDiscard(project, [file.path]), "Discard failed"),
  });
}

/** Throw away every unstaged change, after asking. `count` only words the question. */
export function discardAll(count: number): void {
  useUiStore.getState().askConfirm({
    title: "Discard all changes?",
    destructive: true,
    description: `The unstaged changes to ${count === 1 ? "1 file" : `${count} files`} will be lost, and untracked files deleted. Staged changes are kept.`,
    action: "Discard all",
    onConfirm: () => changeIndex((project) => backend.gitDiscardAll(project), "Discard failed"),
  });
}

/** The identity from Settings, or null to let git config decide. */
function commitAuthor(): GitIdentity | null {
  const name = useSettingsStore.getState().commitIdentity.name.trim();
  const email = useSettingsStore.getState().commitIdentity.email.trim();
  return name && email ? { name, email } : null;
}

/** Commit what is staged. Resolves to whether it went through. */
export async function commit(message: string): Promise<boolean> {
  const project = currentProject();
  if (!project) return false;
  const committed = await reportErrors(async () => {
    const summary = await backend.gitCommit(project, message, commitAuthor());
    toast.success(`Committed ${summary.id}`, { description: summary.summary });
    return true;
  }, "Commit failed");
  await refreshGitStatus();
  return committed ?? false;
}

export async function loadBranches(): Promise<void> {
  const project = currentProject();
  if (!project) return;
  const branches = await warnQuietly(() => backend.gitBranches(project), "Couldn't list branches");
  if (branches) useGitStore.getState().branchesLoaded(branches);
}

/** After HEAD moves, files on disk may differ: re-read everything shown. */
async function afterSwitch(): Promise<void> {
  await Promise.all([refreshGitStatus(), loadBranches(), reportErrors(refreshEntries), reportErrors(refreshEnvs)]);
}

/** Ask for a name and branch off the current commit, switching to the new branch. */
export function createBranch(): void {
  useUiStore.getState().askPrompt({
    title: "New branch",
    description: "Branches off the current commit and switches to it. Uncommitted changes come along.",
    initial: "",
    placeholder: "feature/login",
    onSubmit: async (name) => {
      const project = currentProject();
      if (!project || !name) return false;
      const created = await reportErrors(async () => {
        await backend.gitCreateBranch(project, name, true);
        return true;
      }, "Couldn't create branch");
      if (!created) return false;
      toast.success(`Switched to new branch ${name}`);
      await afterSwitch();
      return true;
    },
  });
}

/**
 * Check out `name`. Switching rewrites files on disk, so unsaved edits are
 * saved first or kept as edits that clash with the new files.
 */
export function switchBranch(name: string): void {
  const run = async () => {
    const project = currentProject();
    if (!project) return;
    const switched = await reportErrors(async () => {
      await backend.gitSwitchBranch(project, name);
      return true;
    }, "Couldn't switch branch");
    if (switched) toast.success(`Switched to ${name}`);
    await afterSwitch();
  };
  if (!hasUnsavedChanges()) {
    void run();
    return;
  }
  useUiStore.getState().askConfirm({
    title: "Unsaved changes",
    description: `Some requests have unsaved edits. Save them to take them to ${name}, or switch and keep them as unsaved edits.`,
    action: "Save all and switch",
    onConfirm: async () => {
      if (await saveAll()) await run();
    },
    alternative: { action: "Switch anyway", onConfirm: run },
  });
}

export function deleteBranch(branch: BranchInfo): void {
  useUiStore.getState().askConfirm({
    title: `Delete ${branch.name}?`,
    destructive: true,
    description: branch.merged
      ? "Its commits are all on the current branch, so nothing is lost."
      : "It has commits that aren't on the current branch. Deleting it loses them.",
    action: "Delete branch",
    onConfirm: async () => {
      const project = currentProject();
      if (!project) return;
      await reportErrors(() => backend.gitDeleteBranch(project, branch.name, !branch.merged), "Couldn't delete branch");
      await loadBranches();
    },
  });
}
