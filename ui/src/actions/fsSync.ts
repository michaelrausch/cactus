import { isCritterPath } from "@/domain/critterId";
import { isEnvFile } from "@/domain/envFiles";
import { reportErrors } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { refreshCritters } from "./critters";
import { flowFilesChanged } from "./flows";
import { refreshGitStatus } from "./git";
import { refreshSchedules } from "./monitors";
import { refreshEntries, refreshEnvs } from "./project";
import { refreshVariableNames } from "./variables";
import { isOwnRecentWrite, reloadIfChangedOnDisk } from "./saving";

/**
 * React to files changing on disk, whoever changed them. Git status is
 * refreshed for every change, saves included.
 *
 * Open requests and the open flow always follow the disk: that compares the
 * file with what was last saved, so our own save's echo changes nothing
 * however late it arrives, while an outside edit made right after a save is
 * still caught. Re-listing the project re-parses every file, so it is skipped
 * for changes that are most likely our own saves.
 */
export async function handleFilesChanged(changed: string[]): Promise<void> {
  if (!useProjectStore.getState().project) return;
  void refreshGitStatus();
  void reportErrors(() => flowFilesChanged(changed));
  await reloadIfChangedOnDisk(changed);
  const external = changed.filter((path) => !isOwnRecentWrite(path));
  if (external.length === 0) return;
  void reportErrors(refreshEntries);
  if (external.some(isEnvFile)) void reportErrors(refreshEnvs);
  if (external.some(isCritterPath)) {
    void reportErrors(refreshCritters);
    void refreshSchedules();
  }
  // Folder and collection files carry variables too, so any change may matter.
  void refreshVariableNames();
}
