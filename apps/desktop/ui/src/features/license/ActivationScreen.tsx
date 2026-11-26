import { useId, useState } from "react";
import { Palette, ShieldCheck, Sparkles, Webhook, type LucideIcon } from "lucide-react";
import { activateLicense } from "@/actions/license";
import { backend } from "@/api/client";
import { Logo } from "@/components/Logo";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { ErrorLine, StackedForm, useRunner } from "@/features/vault/PasswordFields";
import { reportErrors } from "@/lib/errors";

const FEATURES: { icon: LucideIcon; name: string; detail: string }[] = [
  {
    icon: ShieldCheck,
    name: "Security tooling",
    detail: "Fuzzing, security headers, certificates, rate limits and load testing",
  },
  { icon: Palette, name: "Extra themes", detail: "24 more, from Synthwave to Windows 95" },
  { icon: Sparkles, name: "Early access", detail: "New features land in Pro first" },
  { icon: Webhook, name: "Slack and webhooks", detail: "Post a flow's results to Slack or any URL" },
];

const openProPage = () => void reportErrors(() => backend.openProPage(), "Couldn't open the website");

/** Covers the whole Pro app until a license key is entered: what Pro adds, and where the key goes. */
export function ActivationScreen() {
  return (
    <div className="flex h-full flex-col">
      {/* The window's title bar is hidden behind the app, so it needs its own drag strip. */}
      <div data-tauri-drag-region className="h-10 shrink-0" />
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto px-8 pb-10">
        {/* Centred in the window, and the two columns on each other. */}
        <div className="m-auto flex w-full max-w-5xl flex-col gap-10 lg:flex-row lg:items-center lg:gap-14">
          <section className="flex flex-1 flex-col gap-8">
            <div className="flex items-center gap-4">
              {/* The mark has a margin inside its box; pulled left to line up with the icons below. */}
              <Logo className="-ml-1.5 size-14 drop-shadow-[0_0_18px_color-mix(in_oklch,var(--color-brand-2)_40%,transparent)]" />
              <div className="flex flex-col gap-1">
                <h1 className="text-brand text-4xl font-semibold tracking-tight">cactus pro</h1>
                <p className="text-muted-foreground">Everything in cactus, and more.</p>
              </div>
            </div>
            <ul className="grid grid-cols-1 gap-x-8 gap-y-6 sm:grid-cols-2">
              {FEATURES.map(({ icon: Icon, name, detail }) => (
                <li key={name} className="flex items-start gap-3.5">
                  <span className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-brand-2/15 text-brand-2 ring-1 ring-brand-2/25">
                    <Icon className="size-4" />
                  </span>
                  <span className="flex flex-col gap-1">
                    <span className="text-sm font-semibold tracking-tight">{name}</span>
                    <span className="text-xs leading-relaxed text-pretty text-muted-foreground">{detail}</span>
                  </span>
                </li>
              ))}
            </ul>
          </section>
          <KeyCard />
        </div>
      </div>
    </div>
  );
}

function KeyCard() {
  const id = useId();
  const [key, setKey] = useState("");
  const { run, busy, error } = useRunner();
  const ready = !busy && key.trim() !== "";
  return (
    <section className="flex w-full shrink-0 flex-col gap-4 rounded-2xl border bg-card/80 p-6 shadow-lg shadow-shadow/20 lg:w-96">
      <StackedForm onSubmit={() => ready && run(() => activateLicense(key))}>
        <div className="flex flex-col gap-1.5">
          <label htmlFor={id} className="text-xs text-muted-foreground">
            License key
          </label>
          <Textarea
            id={id}
            value={key}
            autoFocus
            spellCheck={false}
            placeholder="CACTUS-PRO-…"
            className="field-sizing-fixed h-20 resize-none px-3 py-3 font-mono text-xs leading-relaxed break-all md:text-xs"
            onChange={(event) => setKey(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                event.currentTarget.form?.requestSubmit();
              }
            }}
          />
        </div>
        <ErrorLine error={error} />
        <Button type="submit" className="mt-1" disabled={!ready}>
          Activate
        </Button>
      </StackedForm>
      <div className="flex items-center justify-between border-t pt-4 text-sm">
        <span className="text-muted-foreground">No key yet?</span>
        <Button variant="outline" size="sm" onClick={openProPage}>
          Buy a license
        </Button>
      </div>
    </section>
  );
}
