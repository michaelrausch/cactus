import { Lock, LockOpen } from "lucide-react";
import { ensureUnlocked, lockSecrets } from "@/actions/secrets";
import { IconButton } from "@/components/IconButton";
import { useSecretsStore } from "@/stores/secretsStore";

/** Beside the project switcher while encryption is on: lock now, or unlock. */
export function LockButton() {
  const status = useSecretsStore((s) => s.info?.status ?? "off");
  if (status === "off") return null;
  return status === "locked" ? (
    <IconButton label="Unlock secrets" tone="sidebar" onClick={() => void ensureUnlocked({ force: true })}>
      <Lock />
    </IconButton>
  ) : (
    <IconButton label="Lock secrets" tone="sidebar" onClick={() => void lockSecrets()}>
      <LockOpen />
    </IconButton>
  );
}
