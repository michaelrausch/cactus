const RELEASE = "https://github.com/michaelrausch/cactus/releases/download/v1.3.0";

export const downloads = {
  macos: `${RELEASE}/cactus_1.3.0_universal.dmg`,
  windowsInstaller: `${RELEASE}/cactus_1.3.0_x64-setup.exe`,
  windowsStandalone: `${RELEASE}/cactus_v1.3.0_x64_portable.exe`,
};

/** The Pro edition's builds, as `.github/workflows/release.yml` names them. */
export const proDownloads = {
  macos: `${RELEASE}/cactus-pro_1.3.0_universal.dmg`,
  windowsInstaller: `${RELEASE}/cactus-pro_1.3.0_x64-setup.exe`,
  windowsStandalone: `${RELEASE}/cactus-pro_v1.3.0_x64_portable.exe`,
};

/** Where Pro licenses come from: any donation here emails a key (worker/index.ts). */
export const KOFI_URL = "https://ko-fi.com/O4F82BBNNS";

/** The cactus pro product in the Ko-fi shop: buying it emails a key. */
export const KOFI_PRO_URL = "https://ko-fi.com/s/91f9b78594";
