/** Schedules and countdowns. Intervals are written as the backend parses them (`Interval::parse`). */

/** `"in 3:07"` until `at`, or `"due"` once it has passed. */
export function countdown(at: number, now: number): string {
  const seconds = Math.ceil((at - now) / 1000);
  if (seconds <= 0) return "due";
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = String(seconds % 60).padStart(2, "0");
  return hours > 0 ? `in ${hours}:${String(minutes).padStart(2, "0")}:${rest}` : `in ${minutes}:${rest}`;
}
