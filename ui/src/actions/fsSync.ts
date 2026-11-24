import { isEnvFile } from "@/domain/envFiles";
import { reportErrors } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { refreshEntries, refreshEnvs } from "./project";
import { reloadIfChangedOnDisk } from "./scenario";

/** React to files changing on disk, whoever changed them. */
export async function handleFilesChanged(paths: string[]): Promise<void> {
  if (!useProjectStore.getState().project) return;
  void reportErrors(refreshEntries);
  if (paths.some(isEnvFile)) void reportErrors(refreshEnvs);
  await reloadIfChangedOnDisk(paths);
}
