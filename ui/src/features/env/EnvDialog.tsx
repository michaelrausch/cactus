import { useRef, useState } from "react";
import { backend, type EnvFile } from "@/api/client";
import { refreshEnvs } from "@/actions/project";
import { refreshVariableNames } from "@/actions/variables";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { envFileKey, envLayerInfo } from "@/domain/envFiles";
import { reportErrors } from "@/lib/errors";
import { useProjectStore } from "@/stores/projectStore";
import { useUiStore } from "@/stores/uiStore";
import { EnvLayerEditor, type EnvLayerEditorHandle } from "./EnvLayerEditor";
import { EnvLayerList } from "./EnvLayerList";

/** Edit every env layer of the open project. */
export function EnvDialog() {
  const project = useProjectStore((s) => s.project);
  const envs = useProjectStore((s) => s.envs);
  const open = useUiStore((s) => s.envDialogOpen);
  const setOpen = useUiStore((s) => s.setEnvDialogOpen);
  const [selected, setSelected] = useState<EnvFile>({ kind: "base" });
  const editor = useRef<EnvLayerEditorHandle>(null);

  if (!project) return null;

  /** Run `leave` once unsaved edits are saved or discarded, or right away if there are none. */
  const guardUnsaved = (leave: () => void) => {
    if (!editor.current?.hasUnsavedChanges()) {
      leave();
      return;
    }
    const { path } = envLayerInfo(selected);
    useUiStore.getState().askConfirm({
      title: "Save changes?",
      description: `You have unsaved changes to ${path}.`,
      action: "Save",
      onConfirm: async () => {
        if (await editor.current?.save()) leave();
      },
      alternative: { action: "Discard", onConfirm: leave, destructive: true },
    });
  };

  const close = () => {
    setOpen(false);
    void refreshVariableNames();
  };

  const select = (file: EnvFile) => {
    if (envFileKey(file) !== envFileKey(selected)) guardUnsaved(() => setSelected(file));
  };

  const createEnv = async (name: string): Promise<boolean> => {
    const file: EnvFile = { kind: "named", name };
    const created = await reportErrors(async () => {
      await backend.saveEnv(project, file, {});
      await refreshEnvs();
      return true;
    });
    if (created) guardUnsaved(() => setSelected(file));
    return created ?? false;
  };

  const renameEnv = (from: string) =>
    guardUnsaved(() =>
      useUiStore.getState().askPrompt({
        title: "Rename environment",
        description: "Letters, digits, - and _ only. It becomes the file name.",
        initial: from,
        onSubmit: async (to) => {
          if (!to) return false;
          if (to === from) return true;
          const renamed = await reportErrors(async () => {
            await backend.renameEnv(project, from, to);
            await refreshEnvs();
            setSelected({ kind: "named", name: to });
            return true;
          }, "Rename failed");
          return renamed ?? false;
        },
      }),
    );

  const deleteEnv = (name: string) =>
    useUiStore.getState().askConfirm({
      title: `Delete ${name}?`,
      destructive: true,
      description: `environments/${name}.yml will be deleted from disk.`,
      action: "Delete",
      onConfirm: () =>
        void reportErrors(async () => {
          await backend.deleteEnv(project, name);
          await refreshEnvs();
          setSelected({ kind: "base" });
        }, "Delete failed"),
    });

  return (
    <Dialog open={open} onOpenChange={(next) => (next ? setOpen(true) : guardUnsaved(close))}>
      <DialogContent className="gap-0 p-0 sm:max-w-3xl">
        <div className="flex h-[min(480px,calc(100vh-4rem))]">
          <EnvLayerList envs={envs} selected={selected} onSelect={select} onCreate={createEnv} />
          {open && (
            <EnvLayerEditor
              key={`${project}:${envFileKey(selected)}`}
              ref={editor}
              project={project}
              file={selected}
              onRename={renameEnv}
              onDelete={deleteEnv}
            />
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
