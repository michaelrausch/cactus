const METHOD_COLORS: Record<string, string> = {
  GET: "text-method-get",
  POST: "text-method-post",
  PUT: "text-method-put",
  PATCH: "text-method-patch",
  DELETE: "text-method-delete",
};
const DEFAULT_METHOD_COLOR = "text-muted-foreground";

export const methodColor = (method: string): string => METHOD_COLORS[method.toUpperCase()] ?? DEFAULT_METHOD_COLOR;

/** Upper bounds (exclusive) of each status class, checked in order. */
const STATUS_COLORS: readonly { below: number; color: string }[] = [
  { below: 300, color: "text-success" },
  { below: 400, color: "text-info" },
  { below: 500, color: "text-warning" },
];
const SERVER_ERROR_COLOR = "text-danger";

export const statusColor = (status: number): string =>
  STATUS_COLORS.find(({ below }) => status < below)?.color ?? SERVER_ERROR_COLOR;
