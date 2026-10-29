const METHOD_COLORS: Record<string, string> = {
  GET: "text-emerald-400",
  POST: "text-amber-400",
  PUT: "text-sky-400",
  PATCH: "text-violet-400",
  DELETE: "text-rose-400",
};
const DEFAULT_METHOD_COLOR = "text-muted-foreground";

export const methodColor = (method: string): string => METHOD_COLORS[method.toUpperCase()] ?? DEFAULT_METHOD_COLOR;

/** Upper bounds (exclusive) of each status class, checked in order. */
const STATUS_COLORS: readonly { below: number; color: string }[] = [
  { below: 300, color: "text-emerald-400" },
  { below: 400, color: "text-sky-400" },
  { below: 500, color: "text-amber-400" },
];
const SERVER_ERROR_COLOR = "text-rose-400";

export const statusColor = (status: number): string =>
  STATUS_COLORS.find(({ below }) => status < below)?.color ?? SERVER_ERROR_COLOR;
