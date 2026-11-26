import { useEffect, useState } from "react";
import { PRO } from "@edition";
import { getTauriVersion } from "@tauri-apps/api/app";
import { isTauri } from "@tauri-apps/api/core";
import { Copy, RotateCcw } from "lucide-react";
import { copyText } from "@/actions/clipboard";
import { showOnboarding } from "@/actions/project";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";
import { useEditionStore } from "@/stores/editionStore";

const APP_NAME = PRO ? "cactus pro" : "cactus";
const buildDate = new Date(import.meta.env.BUILD_DATE);
const currentYear = new Date().getFullYear();

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
  const licensee = useEditionStore((s) => s.licensee);
  const rows: [string, string][] = [
    ...(licensee ? [["Licensed to", licensee] as [string, string]] : []),
    ["Version", import.meta.env.APP_VERSION],
    ["Commit", import.meta.env.GIT_DIRTY ? `${import.meta.env.GIT_SHA} (modified)` : import.meta.env.GIT_SHA],
    ["Built", buildDate.toLocaleString()],
    ["Tauri", tauriVersion ?? "not running in Tauri"],
    ["Platform", navigator.platform || "unknown"],
  ];

  const copy = () => {
    const text = rows.map(([label, value]) => `${label}: ${value}`).join("\n");
    void copyText(`${APP_NAME}\n${text}`, "Build info copied");
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-6 text-center">
      <div className="flex flex-col items-center gap-3">
        <Logo className="size-20 drop-shadow-[0_0_18px_color-mix(in_oklch,var(--color-brand-2)_40%,transparent)]" />
        <h2 className="text-brand text-3xl font-semibold tracking-tight">{APP_NAME}</h2>
      </div>
      <dl className="grid grid-cols-[auto_auto] gap-x-6 gap-y-1.5 rounded-xl border bg-card/60 px-5 py-4 text-left text-sm">
        {rows.map(([label, value]) => (
          <div key={label} className="contents">
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="font-mono text-xs leading-5 tabular-nums">{value}</dd>
          </div>
        ))}
      </dl>
      <div className="flex gap-1">
        <Button variant="ghost" size="sm" onClick={copy}>
          <Copy /> Copy build info
        </Button>
        <Button variant="ghost" size="sm" onClick={showOnboarding}>
          <RotateCcw /> Show onboarding
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">© Michael Rausch {currentYear}</p>
    </div>
  );
}
