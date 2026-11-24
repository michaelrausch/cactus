/** Schedules and countdowns. Intervals are written as the backend parses them (`Interval::parse`). */

/** The shortest schedule the backend accepts. */
export const MIN_INTERVAL_SECONDS = 30;

const UNIT_SECONDS: Record<string, number> = { "": 1, s: 1, m: 60, h: 3600 };

/** `"30s"`, `"5m"`, `"1h"` or bare seconds → seconds; null if invalid or under 30s. Mirrors `Interval::parse`. */
export function parseInterval(text: string): number | null {
  const match = /^(\d+)\s*([smh]?)$/.exec(text.trim());
  if (!match) return null;
  const seconds = Number(match[1]) * (UNIT_SECONDS[match[2] ?? ""] ?? 1);
  return Number.isSafeInteger(seconds) && seconds >= MIN_INTERVAL_SECONDS && seconds <= 0xffffffff ? seconds : null;
}

/** Seconds as the backend writes them: `"1h"`, `"5m"`, `"90s"`. */
export function formatInterval(seconds: number): string {
  if (seconds % 3600 === 0) return `${seconds / 3600}h`;
  if (seconds % 60 === 0) return `${seconds / 60}m`;
  return `${seconds}s`;
}

/** `"in 3:07"` until `at`, or `"due"` once it has passed. */
export function countdown(at: number, now: number): string {
  const seconds = Math.ceil((at - now) / 1000);
  if (seconds <= 0) return "due";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = String(seconds % 60).padStart(2, "0");
  return hours > 0 ? `in ${hours}:${String(minutes).padStart(2, "0")}:${rest}` : `in ${minutes}:${rest}`;
}
