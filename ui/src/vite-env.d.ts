/// <reference types="vite/client" />

/** Build info, set by `define` in vite.config.ts. */
interface ImportMetaEnv {
  readonly APP_VERSION: string;
  readonly GIT_SHA: string;
  readonly GIT_DIRTY: boolean;
  readonly BUILD_DATE: string;
  /** Whether the secret encryption UI is built in (`CACTUS_ENCRYPTION=1`). */
  readonly SECRETS_ENCRYPTION: boolean;
}
