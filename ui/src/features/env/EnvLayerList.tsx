import { useState } from "react";
import { Lock, Plus, Sparkles } from "lucide-react";
import type { EnvFile } from "@/api/client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { envFileKey, envLayerInfo, envLayers } from "@/domain/envFiles";
import { cn } from "@/lib/utils";

function LayerIcon({ file }: { file: EnvFile }) {
  switch (file.kind) {
    case "secrets":
      return <Lock className="size-3.5 text-warning" />;
    case "captured":
      return <Sparkles className="size-3.5 text-captured" />;
    case "base":
      return <span className="size-2 rounded-full bg-muted-foreground/60" />;
    case "named":
      return <span className="size-2 rounded-full bg-brand" />;
  }
}

function NewEnvForm({ onCreate }: { onCreate: (name: string) => Promise<boolean> }) {
  const [name, setName] = useState("");
  return (
    <form
      className="mt-auto flex gap-1 pt-2"
      onSubmit={(e) => {
        e.preventDefault();
        const trimmed = name.trim();
        if (!trimmed) return;
        void onCreate(trimmed).then((created) => created && setName(""));
      }}
    >
      <Input
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="New environment"
        aria-label="New environment name"
        className="h-7 text-xs"
      />
      <Button
        type="submit"
        size="icon"
        variant="ghost"
        className="size-7 shrink-0"
        title="Create environment"
        aria-label="Create environment"
      >
        <Plus />
      </Button>
    </form>
  );
}

interface EnvLayerListProps {
  envs: string[];
  selected: EnvFile;
  onSelect: (file: EnvFile) => void;
  /** Resolves to whether the environment was created. */
  onCreate: (name: string) => Promise<boolean>;
}

export function EnvLayerList({ envs, selected, onSelect, onCreate }: EnvLayerListProps) {
  return (
    <div className="flex w-52 shrink-0 flex-col gap-0.5 border-r bg-sidebar p-2">
      {envLayers(envs).map((file) => (
        <button
          type="button"
          key={envFileKey(file)}
          onClick={() => onSelect(file)}
          aria-current={envFileKey(file) === envFileKey(selected) || undefined}
          className={cn(
            "focus-ring",
            "flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-sidebar-accent",
            envFileKey(file) === envFileKey(selected) && "bg-sidebar-accent",
          )}
        >
          <LayerIcon file={file} />
          <span className="truncate">{envLayerInfo(file).label}</span>
        </button>
      ))}
      <NewEnvForm onCreate={onCreate} />
    </div>
  );
}
