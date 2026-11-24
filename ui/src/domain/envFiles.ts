import type { EnvFile } from "@/api/client";

const COLLECTION_FILE = "opencollection.yml";
/** Mirrors `layout::environment_file` in Rust. */
const ENVIRONMENT_FILE = /^environments\/[^/]+\.yml$/;

/** Whether a change to `path` can change the env layers. */
export const isEnvFile = (path: string): boolean => path === COLLECTION_FILE || ENVIRONMENT_FILE.test(path);

export interface EnvLayerInfo {
  label: string;
  path: string;
}

const FIXED_LAYERS: Record<Exclude<EnvFile["kind"], "named">, EnvLayerInfo> = {
  base: {
    label: "Shared",
    path: COLLECTION_FILE,
  },
  secrets: {
    label: "Secrets",
    path: ".cactus/secrets.json",
  },
  captured: {
    label: "Captured",
    path: ".cactus/captured.json",
  },
};

export function envLayerInfo(file: EnvFile): EnvLayerInfo {
  if (file.kind !== "named") return FIXED_LAYERS[file.kind];
  return { label: file.name, path: `environments/${file.name}.yml` };
}

export const envFileKey = (file: EnvFile): string => (file.kind === "named" ? `named:${file.name}` : file.kind);

/** Every layer, lowest precedence first, as the backend merges them. */
export const envLayers = (namedEnvs: string[]): EnvFile[] => [
  { kind: "base" },
  ...namedEnvs.map((name): EnvFile => ({ kind: "named", name })),
  { kind: "secrets" },
  { kind: "captured" },
];
