import type { EnvFile } from "@/api/client";

/** Mirrors `layout::is_env_file` in Rust, for paths at the project root. */
const ROOT_ENV_FILE = /^env(\.[^/]+)?\.json$/;

export const isRootEnvFile = (path: string): boolean => ROOT_ENV_FILE.test(path);

export interface EnvLayerInfo {
  label: string;
  path: string;
  note: string;
}

const FIXED_LAYERS: Record<Exclude<EnvFile["kind"], "named">, EnvLayerInfo> = {
  base: { label: "Shared", path: "env.json", note: "Committed. Values every environment starts from." },
  secrets: {
    label: "Secrets",
    path: ".cactus/secrets.json",
    note: "Never committed. Overrides the active environment: put tokens and passwords here.",
  },
  captured: {
    label: "Captured",
    path: ".cactus/captured.json",
    note: "Written by Capture rules after each send. Highest precedence.",
  },
};

export function envLayerInfo(file: EnvFile): EnvLayerInfo {
  if (file.kind !== "named") return FIXED_LAYERS[file.kind];
  return { label: file.name, path: `env.${file.name}.json`, note: "Committed. Overrides Shared when active." };
}

export const envFileKey = (file: EnvFile): string => (file.kind === "named" ? `named:${file.name}` : file.kind);

/** Every layer, lowest precedence first, as the backend merges them. */
export const envLayers = (namedEnvs: string[]): EnvFile[] => [
  { kind: "base" },
  ...namedEnvs.map((name): EnvFile => ({ kind: "named", name })),
  { kind: "secrets" },
  { kind: "captured" },
];
