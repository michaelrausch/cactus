import { Check } from "lucide-react";
import { Logo } from "@/components/Logo";
import { cn } from "@/lib/utils";
import { useSettingsStore } from "@/stores/settingsStore";
import { THEME_GROUPS, THEMES, type ThemeEntry } from "@/theme/themes";

/**
 * A tiny mock of the app. It carries `data-theme` itself, so it is drawn with
 * the theme's real tokens whatever theme the app is in.
 */
function ThemePreview({ theme }: { theme: ThemeEntry }) {
  return (
    <div
      data-theme={theme.id}
      className="relative flex h-24 overflow-hidden rounded-(--radius) border bg-background bg-(image:--app-backdrop) font-sans text-foreground"
    >
      <div className="flex w-12 shrink-0 flex-col gap-1.5 bg-sidebar p-1.5">
        <Logo className="size-3.5" />
        <div className="h-1.5 rounded-full bg-sidebar-accent" />
        <div className="h-1.5 w-3/4 rounded-full bg-sidebar-accent/70" />
        <div className="h-1.5 w-5/6 rounded-full bg-sidebar-accent/70" />
      </div>
      <div className="flex min-w-0 flex-1 flex-col gap-1.5 p-1.5">
        <div className="flex items-center gap-1 rounded-(--radius) border bg-card px-1 py-0.5">
          <span className="font-mono text-[8px] font-semibold text-method-get">GET</span>
          <div className="h-1 flex-1 rounded-full bg-muted" />
          <div className="size-2.5 rounded-[3px] bg-brand-2" />
        </div>
        <div className="flex-1 rounded-(--radius) border bg-card/70 px-1.5 py-1 font-mono text-[8px] leading-tight">
          <div>
            <span className="text-syntax-key">"id"</span>: <span className="text-syntax-number">42</span>
          </div>
          <div>
            <span className="text-syntax-key">"ok"</span>: <span className="text-syntax-literal">true</span>
          </div>
          <div className="truncate">
            <span className="text-syntax-key">"name"</span>: <span className="text-syntax-string">"saguaro"</span>
          </div>
        </div>
        <div className="flex items-center gap-1">
          {["bg-success", "bg-info", "bg-warning", "bg-danger"].map((color) => (
            <span key={color} className={cn("size-1.5 rounded-full", color)} />
          ))}
          <div className="ml-auto h-1.5 w-8 rounded-full bg-brand" />
        </div>
      </div>
      <div aria-hidden className="pointer-events-none absolute inset-0 bg-(image:--app-overlay)" />
    </div>
  );
}

function ThemeCard({ theme, selected, onSelect }: { theme: ThemeEntry; selected: boolean; onSelect: () => void }) {
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      onClick={onSelect}
      className={cn(
        "group flex flex-col gap-2 rounded-xl p-1.5 text-left ring-1 ring-transparent transition outline-none hover:bg-accent/50 focus-visible:ring-ring",
        selected && "bg-accent/60 ring-brand-2/70",
      )}
    >
      <ThemePreview theme={theme} />
      <div className="flex items-center gap-1.5 px-1 pb-0.5">
        <p className="min-w-0 flex-1 truncate text-sm font-medium">{theme.name}</p>
        {selected && <Check className="size-4 shrink-0 text-brand-2" />}
      </div>
    </button>
  );
}

export function ThemeTab() {
  const themeId = useSettingsStore((s) => s.themeId);
  const setTheme = useSettingsStore((s) => s.setTheme);

  return (
    <div role="radiogroup" aria-label="Theme" className="flex flex-col gap-5">
      {THEME_GROUPS.map(({ group, label }) => (
        <section key={group} className="flex flex-col gap-2">
          <h3 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{label}</h3>
          <div className="grid grid-cols-2 gap-2 lg:grid-cols-3">
            {THEMES.filter((theme) => theme.group === group).map((theme) => (
              <ThemeCard
                key={theme.id}
                theme={theme}
                selected={theme.id === themeId}
                onSelect={() => setTheme(theme.id)}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
