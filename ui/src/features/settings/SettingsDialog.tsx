import type { ReactNode } from "react";
import { Info, Palette } from "lucide-react";
import { Tabs } from "radix-ui";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { useUiStore } from "@/stores/uiStore";
import { AboutTab } from "./AboutTab";
import { ThemeTab } from "./ThemeTab";

/** A row in the settings nav: highlighted when active, no underline or side bar. */
function NavItem({ value, children }: { value: string; children: ReactNode }) {
  return (
    <Tabs.Trigger
      value={value}
      className="flex h-8 items-center gap-2 rounded-md px-2 text-sm text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-[state=active]:bg-sidebar-accent data-[state=active]:text-foreground [&_svg]:size-4"
    >
      {children}
    </Tabs.Trigger>
  );
}

/** Opened from the cactus in the sidebar, or Ctrl+,. */
export function SettingsDialog() {
  const open = useUiStore((s) => s.settingsOpen);
  const setOpen = useUiStore((s) => s.setSettingsOpen);

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="gap-0 p-0 sm:max-w-3xl">
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">Theme and information about this build.</DialogDescription>
        <Tabs.Root orientation="vertical" defaultValue="theme" className="flex h-[520px]">
          <div className="flex w-44 shrink-0 flex-col gap-3 border-r bg-sidebar/60 p-3">
            <p className="px-2 pt-1 text-xs font-medium tracking-wide text-muted-foreground uppercase">Settings</p>
            <Tabs.List className="flex flex-col gap-0.5">
              <NavItem value="theme">
                <Palette /> Theme
              </NavItem>
              <NavItem value="about">
                <Info /> About
              </NavItem>
            </Tabs.List>
          </div>
          <Tabs.Content value="theme" className="min-h-0 flex-1 overflow-y-auto p-5 outline-none">
            <ThemeTab />
          </Tabs.Content>
          <Tabs.Content value="about" className="min-h-0 flex-1 overflow-y-auto p-5 outline-none">
            <AboutTab />
          </Tabs.Content>
        </Tabs.Root>
      </DialogContent>
    </Dialog>
  );
}
