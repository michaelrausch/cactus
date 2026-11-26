/// <reference types="vite/client" />

/** Build info, set by `define` in vite.config.ts. */
interface ImportMetaEnv {
  readonly APP_VERSION: string;
  readonly GIT_SHA: string;
  readonly GIT_DIRTY: boolean;
  readonly BUILD_DATE: string;
}
