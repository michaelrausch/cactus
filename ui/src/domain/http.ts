import type { Auth, BodyType } from "@/api/client";

export const HTTP_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] as const;

export const BODY_TYPES: readonly BodyType[] = ["none", "json", "text", "form", "multipart"];

export type AuthKind = Auth["type"] | "none";

export const AUTH_KIND_LABELS: Record<AuthKind, string> = {
  none: "No auth",
  bearer: "Bearer token",
  basic: "Basic auth",
};

/** What a freshly chosen auth type starts as; `none` removes auth. */
export const AUTH_DEFAULTS: Record<AuthKind, Auth | null> = {
  none: null,
  bearer: { type: "bearer", token: "" },
  basic: { type: "basic", username: "", password: "" },
};

export const isAuthKind = (value: string): value is AuthKind => value in AUTH_KIND_LABELS;
