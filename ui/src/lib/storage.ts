import { PRO } from "@edition/flag";

/**
 * A localStorage key, apart per edition: in dev both editions share one
 * origin, and a Pro install starts fresh (onboarding included) either way.
 */
export const storageKey = (name: string): string => `${PRO ? "cactus-pro" : "cactus"}.${name}`;
