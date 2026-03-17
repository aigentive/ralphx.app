import { isTauriMode } from "./tauri-detection";

/**
 * WKWebView on macOS is more sensitive to animated scroll updates in our
 * virtualized chat surfaces, so prefer non-animated scrolling there.
 */
export function shouldUseWebkitSafeScrollBehavior(): boolean {
  if (typeof navigator === "undefined") {
    return false;
  }

  const isMacPlatform =
    navigator.platform.includes("Mac") || navigator.userAgent.includes("Mac");

  return isTauriMode() && isMacPlatform;
}
