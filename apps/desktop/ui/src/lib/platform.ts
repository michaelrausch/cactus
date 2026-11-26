/** macOS keeps native traffic lights; other platforms draw their own window controls. */
export const isMac = /Mac/i.test(navigator.userAgent);
