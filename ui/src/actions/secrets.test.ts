import { beforeEach, describe, expect, it } from "vitest";
import type { SecretsStatus } from "@/api/client";
import { useSecretsStore } from "@/stores/secretsStore";
import { ensureUnlocked, forgotPassword, unlockDialogClosed } from "./secrets";

const withStatus = (status: SecretsStatus) =>
  useSecretsStore.getState().loaded({ status, autoLockMinutes: 15, hasSealedValues: true });

describe("ensureUnlocked", () => {
  beforeEach(() => useSecretsStore.getState().reset());

  it("resolves at once when off or unlocked", async () => {
    withStatus("off");
    await expect(ensureUnlocked()).resolves.toBe(true);
    withStatus("unlocked");
    await expect(ensureUnlocked()).resolves.toBe(true);
    expect(useSecretsStore.getState().dialog).toBeNull();
  });

  it("doesn't ask when nothing is encrypted, unless forced", async () => {
    useSecretsStore.getState().loaded({ status: "locked", autoLockMinutes: 15, hasSealedValues: false });
    await expect(ensureUnlocked()).resolves.toBe(true);
    expect(useSecretsStore.getState().dialog).toBeNull();
    const forced = ensureUnlocked({ force: true });
    expect(useSecretsStore.getState().dialog).toBe("unlock");
    unlockDialogClosed(true);
    await expect(forced).resolves.toBe(true);
  });

  it("waits for the unlock dialog when locked", async () => {
    withStatus("locked");
    const first = ensureUnlocked();
    const second = ensureUnlocked();
    expect(useSecretsStore.getState().dialog).toBe("unlock");
    unlockDialogClosed(true);
    await expect(Promise.all([first, second])).resolves.toEqual([true, true]);
    expect(useSecretsStore.getState().dialog).toBeNull();
  });

  it("resolves false when cancelled or sent to reset", async () => {
    withStatus("locked");
    const cancelled = ensureUnlocked();
    unlockDialogClosed(false);
    await expect(cancelled).resolves.toBe(false);

    const forgot = ensureUnlocked();
    forgotPassword();
    await expect(forgot).resolves.toBe(false);
    expect(useSecretsStore.getState().dialog).toBe("reset");
  });
});
