import type { ScenarioTabProps } from "../types";

/** Scripts come from Bruno or Postman. cactus keeps them in the file but can't run them. */
export function ScriptsTab({ scenario }: Pick<ScenarioTabProps, "scenario">) {
  const scripts = scenario.runtime?.scripts ?? [];
  return (
    <>
      {scripts.map((script) => (
        <section key={`${script.type}:${script.code}`} className="flex flex-col gap-1.5">
          <span className="text-xs text-muted-foreground">{script.type}</span>
          <pre className="overflow-auto rounded-lg bg-card p-3 font-mono text-xs leading-relaxed">{script.code}</pre>
        </section>
      ))}
    </>
  );
}
