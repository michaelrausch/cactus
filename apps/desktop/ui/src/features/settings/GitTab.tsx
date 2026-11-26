import { useEffect, useId, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { backend, type GitIdentity } from "@/api/client";
import { Input } from "@/components/ui/input";
import { useProjectStore } from "@/stores/projectStore";
import { useSettingsStore, type CommitIdentity } from "@/stores/settingsStore";

/** What git config would sign commits with, for the open project if there is one. */
function useConfiguredIdentity(): GitIdentity | null {
  const project = useProjectStore((s) => s.project);
  const [identity, setIdentity] = useState<GitIdentity | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    let current = true;
    backend
      .gitConfiguredIdentity(project)
      .then((found) => current && setIdentity(found))
      .catch(() => current && setIdentity(null));
    return () => {
      current = false;
    };
  }, [project]);
  return identity;
}

function Field({
  label,
  field,
  placeholder,
  type = "text",
}: {
  label: string;
  field: keyof CommitIdentity;
  placeholder: string;
  type?: string;
}) {
  const id = useId();
  const value = useSettingsStore((s) => s.commitIdentity[field]);
  const setCommitIdentity = useSettingsStore((s) => s.setCommitIdentity);
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={id} className="text-sm font-medium">
        {label}
      </label>
      <Input
        id={id}
        type={type}
        value={value}
        placeholder={placeholder}
        spellCheck={false}
        autoComplete="off"
        onChange={(event) => setCommitIdentity({ [field]: event.target.value })}
      />
    </div>
  );
}

/** Which identity the next commit will use, in a sentence. */
function describeAuthor(name: string, email: string, configured: GitIdentity | null): string {
  if (name && email) return `Committing as ${name} <${email}>.`;
  const fallback = configured
    ? `Committing as ${configured.name} <${configured.email}> from git config.`
    : "git config has no identity, so commits will fail until you fill these in.";
  return name || email ? `Fill in both to use them. ${fallback}` : fallback;
}

export function GitTab() {
  const configured = useConfiguredIdentity();
  const name = useSettingsStore((s) => s.commitIdentity.name.trim());
  const email = useSettingsStore((s) => s.commitIdentity.email.trim());

  return (
    <section className="flex max-w-md flex-col gap-4">
      <div>
        <h3 className="text-xs font-medium tracking-wide text-muted-foreground uppercase">Commit author</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          Commits are signed with this name and email. Leave both blank to use <code>user.name</code> and{" "}
          <code>user.email</code> from git config.
        </p>
      </div>
      <Field label="Name" field="name" placeholder={configured?.name ?? "Your Name"} />
      <Field label="Email" field="email" type="email" placeholder={configured?.email ?? "you@example.com"} />
      <p className="text-xs text-muted-foreground">{describeAuthor(name, email, configured)}</p>
    </section>
  );
}
