import { useState } from "react";
import { backend, type EnvFile } from "@/api/client";
import { refreshEnvs } from "@/actions/project";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { envFileKey } from "@/domain/envFiles";
import { reportErrors } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { EnvLayerEditor } from "./EnvLayerEditor";
import { EnvLayerList } from "./EnvLayerList";

/** Edit every env layer of the open project. */
export function EnvDialog() {
  const project = useProjectStore((s) => s.project);
  const envs = useProjectStore((s) => s.envs);
  const open = useUiStore((s) => s.envDialogOpen);
  const setOpen = useUiStore((s) => s.setEnvDialogOpen);
  const [selected, setSelected] = useState<EnvFile>({ kind: "base" });

  if (!project) return null;

  const createEnv = async (name: string): Promise<boolean> => {
    const file: EnvFile = { kind: "named", name };
    const created = await reportErrors(async () => {
      await backend.saveEnv(project, file, {});
      await refreshEnvs();
      return true;
    });
    if (created) setSelected(file);
    return created ?? false;
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="gap-0 p-0 sm:max-w-3xl">
        <div className="flex h-[480px]">
          <EnvLayerList envs={envs} selected={selected} onSelect={setSelected} onCreate={createEnv} />
          {open && <EnvLayerEditor key={`${project}:${envFileKey(selected)}`} project={project} file={selected} />}
        </div>
      </DialogContent>
    </Dialog>
  );
}
