import type { ChangeKind } from "@/api/client";

const CHANGE_COLORS: Record<ChangeKind, string> = {
  added: "text-success",
  untracked: "text-success",
  modified: "text-warning",
  renamed: "text-info",
  deleted: "text-danger",
  conflicted: "text-danger",
};

export const changeColor = (kind: ChangeKind): string => CHANGE_COLORS[kind];
