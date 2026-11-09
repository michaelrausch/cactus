import { isEnvFile } from "@/domain/envFiles";
import { reportErrors } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { refreshGitStatus } from "./git";
import { refreshEntries, refreshEnvs } from "./project";
import { refreshVariableNames } from "./variables";
import { isOwnRecentWrite, isSaving, reloadIfChangedOnDisk } from "./saving";

/**
 * React to files changing on disk, whoever changed them. Git status is
 * refreshed for every change, saves included.
 *
 * Open buffers always follow the disk: that compares the file with what was
 * last saved, so our own save's echo changes nothing, while an outside edit
 * made right after a save is still caught. Only a save still being read back
 * is skipped, as its buffer hasn't taken in the save yet. Re-listing the
 * project re-parses every file, so it is skipped for changes that are most
 * likely our own saves.
 */
export async function handleFilesChanged(changed: string[]): Promise<void> {
  if (!useProjectStore.getState().project) return;
  void refreshGitStatus();
  await reloadIfChangedOnDisk(changed.filter((path) => !isSaving(path)));
  const external = changed.filter((path) => !isOwnRecentWrite(path));
  if (external.length === 0) return;
  void reportErrors(refreshEntries);
  if (external.some(isEnvFile)) void reportErrors(refreshEnvs);
  // Folder and collection files carry variables too, so any change may matter.
  void refreshVariableNames();
}
