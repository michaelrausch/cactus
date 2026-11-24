import { isEnvFile } from "@/domain/envFiles";
import { reportErrors } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { refreshEntries, refreshEnvs } from "./project";
import { refreshVariableNames } from "./variables";
import { isOwnRecentWrite, reloadIfChangedOnDisk } from "./saving";

/**
 * React to files changing on disk, whoever changed them. Our own
 * saves are skipped: the store already has what was saved, and re-listing
 * the project after every pause in typing re-parses every file in it.
 */
export async function handleFilesChanged(changed: string[]): Promise<void> {
  if (!useProjectStore.getState().project) return;
  const paths = changed.filter((path) => !isOwnRecentWrite(path));
  if (paths.length === 0) return;
  void reportErrors(refreshEntries);
  if (paths.some(isEnvFile)) void reportErrors(refreshEnvs);
  await reloadIfChangedOnDisk(paths);
  // Folder and collection files carry variables too, so any change may matter.
  void refreshVariableNames();
}
