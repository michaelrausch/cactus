/** Matches `cactus_core::secrets::MIN_PASSWORD_CHARS`. */
export const MIN_PASSWORD_CHARS = 8;

/** What's wrong with a new password and its confirmation, or `null`. Length counts characters, not UTF-16 units. */
export function newPasswordProblem(password: string, confirm: string): string | null {
  if ([...password].length < MIN_PASSWORD_CHARS) return `Use at least ${MIN_PASSWORD_CHARS} characters`;
  if (password !== confirm) return "Passwords don't match";
  return null;
}

/** Auto-lock choices in minutes; `null` is never. */
export const AUTO_LOCK_OPTIONS: readonly (number | null)[] = [5, 15, 60, 240, null];

export function autoLockLabel(minutes: number | null): string {
  if (minutes === null) return "Never";
  if (minutes < 60) return `After ${minutes} min`;
  const hours = minutes / 60;
  return `After ${hours} hour${hours === 1 ? "" : "s"}`;
}
