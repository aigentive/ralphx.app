/**
 * Mock implementation of @tauri-apps/plugin-global-shortcut for web mode
 *
 * Global shortcuts can't be registered in browser mode.
 * Provides no-op implementations that resolve successfully.
 */

export type ShortcutHandler = (shortcut: string) => void;

/**
 * Mock register global shortcut - no-op but resolves successfully
 * In browser mode, global shortcuts aren't available, but we don't want
 * to break the app with errors.
 */
export async function register(
  _shortcut: string,
  _handler: ShortcutHandler
): Promise<void> {
  console.debug(`[mock] global-shortcut.register called for: ${_shortcut} - no-op`);
  // Silently succeed - the shortcut just won't work in web mode
}

/**
 * Mock unregister global shortcut - no-op but resolves successfully
 */
export async function unregister(_shortcut: string): Promise<void> {
  console.debug(`[mock] global-shortcut.unregister called for: ${_shortcut} - no-op`);
}

/**
 * Mock unregister all shortcuts - no-op but resolves successfully
 */
export async function unregisterAll(): Promise<void> {
  console.debug("[mock] global-shortcut.unregisterAll called - no-op");
}

/**
 * Mock check if shortcut is registered - returns false
 */
export async function isRegistered(_shortcut: string): Promise<boolean> {
  console.debug(`[mock] global-shortcut.isRegistered called for: ${_shortcut} - returning false`);
  return false;
}
