/**
 * Mock implementation of @tauri-apps/plugin-process for web mode
 *
 * Process control operations cannot work in browser mode.
 * Provides no-op implementations to prevent runtime crashes.
 */

/**
 * Mock relaunch - logs warning and does nothing
 * In browser, we can't restart the app, so this is a no-op
 */
export async function relaunch(): Promise<void> {
  console.warn("[mock] process.relaunch called - cannot restart in browser mode");
  // Could optionally reload the page: window.location.reload()
  // But that's different from a true app restart, so we just no-op
}

/**
 * Mock exit - logs warning and does nothing
 * In browser, we can't exit the process
 */
export async function exit(_exitCode?: number): Promise<void> {
  console.warn("[mock] process.exit called - cannot exit in browser mode");
}
