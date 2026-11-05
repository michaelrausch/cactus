import { useEffect, useState } from "react";
import { getTauriVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { Copy } from "lucide-react";
import { toast } from "sonner";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";

const buildDate = new Date(import.meta.env.BUILD_DATE);

/** The Tauri runtime version, or null in a plain browser (`vite dev`). */
function useTauriVersion(): string | null {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    getTauriVersion()
      .then(setVersion)
      .catch(() => setVersion(null));
  }, []);
  return version;
}

export function AboutTab() {
  const tauriVersion = useTauriVersion();
  const rows: [string, string][] = [
    ["Version", import.meta.env.APP_VERSION],
    ["Commit", import.meta.env.GIT_DIRTY ? `${import.meta.env.GIT_SHA} (modified)` : import.meta.env.GIT_SHA],
    ["Built", buildDate.toLocaleString()],
    ["Tauri", tauriVersion ?? "not running in Tauri"],
    ["Platform", navigator.platform || "unknown"],
  ];

  const copy = () => {
    const text = rows.map(([label, value]) => `${label}: ${value}`).join("\n");
    navigator.clipboard.writeText(`cactus\n${text}`).then(
      () => toast.success("Build info copied"),
      () => toast.error("Couldn't copy to the clipboard"),
    );
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-6 text-center">
      <div className="flex flex-col items-center gap-3">
        <Logo className="size-20 drop-shadow-[0_0_18px_color-mix(in_oklch,var(--color-brand-2)_40%,transparent)]" />
        <div>
          <h2 className="text-brand text-3xl font-semibold tracking-tight">cactus</h2>
          <p className="mt-1 text-sm text-muted-foreground">Requests as plain files. Fast.</p>
        </div>
      </div>
      <dl className="grid grid-cols-[auto_auto] gap-x-6 gap-y-1.5 rounded-xl border bg-card/60 px-5 py-4 text-left text-sm">
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="font-mono text-xs leading-5 tabular-nums">{value}</dd>
          </div>
        ))}
      </dl>
      <Button variant="ghost" size="sm" onClick={copy}>
        <Copy /> Copy build info
      </Button>
    </div>
  );
}
