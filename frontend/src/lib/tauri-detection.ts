/**
 * Tauri Environment Detection
 *
 * Provides utilities to detect whether the app is running in Tauri's WebView
 * or in a standard browser (for testing/web mode).
 *
 * Detection is based on the presence of window.__TAURI_INTERNALS__ which
 * is only available in Tauri's WebView context.
 */

// Extend Window interface to include Tauri internals
declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

/**
 * Check if the app is running in web mode (browser without Tauri)
 *
 * @returns true if running in a standard browser, false if in Tauri WebView
 */
export function isWebMode(): boolean {
  return typeof window !== "undefined" && !window.__TAURI_INTERNALS__;
}

/**
 * Check if the app is running in Tauri mode (WebView with backend)
 *
 * @returns true if running in Tauri WebView, false if in standard browser
 */
export function isTauriMode(): boolean {
  return typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;
}
