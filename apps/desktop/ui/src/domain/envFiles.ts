import type { EnvFile } from "@/api/client";

const COLLECTION_FILE = "opencollection.yml";
/** Mirrors `layout::environment_file` in Rust. */
const ENVIRONMENT_FILE = /^environments\/[^/]+\.yml$/;

/** Whether a change to `path` can change the env layers. */
export const isEnvFile = (path: string): boolean => path === COLLECTION_FILE || ENVIRONMENT_FILE.test(path);

export interface EnvLayerInfo {
  label: string;
  /** The file, for layers stored as plain YAML; null for the encrypted ones in `.cactus/`. */
  path: string | null;
}

type NamedLayer = Extract<EnvFile, { name: string }>;

const FIXED_LAYERS: Record<Exclude<EnvFile["kind"], NamedLayer["kind"]>, EnvLayerInfo> = {
  base: {
    label: "Shared",
    path: COLLECTION_FILE,
  },
  secrets: { label: "Secrets", path: null },
  captured: { label: "Captured", path: null },
};

/** Mirrors `layout::critter_file` and friends in Rust. */
const NAMED_LAYERS: Record<NamedLayer["kind"], (name: string) => EnvLayerInfo> = {
  named: (name) => ({ label: name, path: `environments/${name}.yml` }),
  critter: (name) => ({ label: "Variables", path: `critters/${name}/critter.yml` }),
  critterSecrets: () => ({ label: "Secrets", path: null }),
  critterContext: () => ({ label: "Captured", path: null }),
};

const hasName = (file: EnvFile): file is NamedLayer => "name" in file;

export function envLayerInfo(file: EnvFile): EnvLayerInfo {
  return hasName(file) ? NAMED_LAYERS[file.kind](file.name) : FIXED_LAYERS[file.kind];
}

/** A string id for an env file, for React keys and lookups. */
export const envFileKey = (file: EnvFile): string => (hasName(file) ? `${file.kind}:${file.name}` : file.kind);

/** Every layer, lowest precedence first, as the backend merges them. */
export const envLayers = (namedEnvs: string[]): EnvFile[] => [
  { kind: "base" },
  ...namedEnvs.map((name): EnvFile => ({ kind: "named", name })),
  { kind: "secrets" },
  { kind: "captured" },
];
