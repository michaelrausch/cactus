import { Children, Fragment, useState, type ReactNode } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { SHORTCUTS, type ShortcutId } from "@/app/shortcuts";
import { ShortcutKeys } from "@/components/Kbd";
import { Button } from "@/components/ui/button";
import { GUIDE_PAGES, type GuidePage } from "./guide/pages";

const SHORTCUTS_PAGE = "shortcuts";

/** Keys outside the Ctrl shortcuts table, used in the flow editor. */
const EXTRA_SHORTCUTS: [string, string][] = [
  ["Ctrl Z", "Undo (flows)"],
  ["Ctrl Shift Z", "Redo (flows)"],
];

function Heading({ children }: { children: ReactNode }) {
  return <h3 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{children}</h3>;
}

function Shortcuts() {
  return (
    <div className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 rounded-xl border bg-card/60 px-4 py-3 text-sm">
      {(Object.keys(SHORTCUTS) as ShortcutId[]).map((id) => (
        <Fragment key={id}>
          <ShortcutKeys id={id} className="font-mono text-xs leading-5 text-muted-foreground" />
          <span className="first-letter:uppercase">{SHORTCUTS[id].description}</span>
        </Fragment>
      ))}
      {EXTRA_SHORTCUTS.map(([keys, description]) => (
        <Fragment key={keys}>
          <kbd className="font-mono text-xs leading-5 text-muted-foreground">{keys}</kbd>
          <span>{description}</span>
        </Fragment>
      ))}
    </div>
  );
}

function Page({ page }: { page: GuidePage }) {
  return (
    <div className="flex flex-col gap-5 text-sm leading-relaxed">
      {/* The pages never change, so `Children.toArray`'s positional keys are stable. */}
      {Children.toArray(
        page.sections.map((section) => (
          <section className="flex flex-col gap-1.5">
            {section.heading && <Heading>{section.heading}</Heading>}
            {Children.toArray(section.body.map((paragraph) => <p>{paragraph}</p>))}
          </section>
        )),
      )}
    </div>
  );
}

function Index({ onOpen }: { onOpen: (id: string) => void }) {
  const rows = [...GUIDE_PAGES.map(({ id, title }) => ({ id, title })), { id: SHORTCUTS_PAGE, title: "Shortcuts" }];
  return (
    <div className="flex flex-col gap-2">
      <Heading>Guide</Heading>
      <div className="flex flex-col divide-y overflow-hidden rounded-xl border bg-card/60">
        {rows.map(({ id, title }) => (
          <button
            key={id}
            type="button"
            onClick={() => onOpen(id)}
            className="flex items-center px-4 py-2.5 text-left text-sm outline-none hover:bg-accent/50 focus-visible:bg-accent/50"
          >
            <span className="flex-1">{title}</span>
            <ChevronRight className="size-4 text-muted-foreground" />
          </button>
        ))}
      </div>
    </div>
  );
}

/** A short manual: a list of topics, each its own page. */
export function GuideTab() {
  const [openId, setOpenId] = useState<string | null>(null);
  const page = GUIDE_PAGES.find((p) => p.id === openId);
  if (!openId) return <Index onOpen={setOpenId} />;
  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="icon-sm" onClick={() => setOpenId(null)} aria-label="Back">
          <ChevronLeft />
        </Button>
        <h2 className="text-lg font-semibold tracking-tight">{page?.title ?? "Shortcuts"}</h2>
      </div>
      {page ? <Page page={page} /> : <Shortcuts />}
    </div>
  );
}
