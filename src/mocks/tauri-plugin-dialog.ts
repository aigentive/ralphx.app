/**
 * Mock implementation of @tauri-apps/plugin-dialog for web mode
 *
 * All functions return no-op values since dialogs can't work in browser.
 * This prevents runtime errors when the app runs in web mode.
 */

export interface OpenDialogOptions {
  defaultPath?: string;
  directory?: boolean;
  filters?: { name: string; extensions: string[] }[];
  multiple?: boolean;
  title?: string;
}

export interface SaveDialogOptions {
  defaultPath?: string;
  filters?: { name: string; extensions: string[] }[];
  title?: string;
}

export interface MessageDialogOptions {
  title?: string;
  okLabel?: string;
  cancelLabel?: string;
  kind?: "info" | "warning" | "error";
}

export interface ConfirmDialogOptions {
  title?: string;
  okLabel?: string;
  cancelLabel?: string;
  kind?: "info" | "warning" | "error";
}

/**
 * Mock open dialog - returns null (user cancelled)
 */
export async function open(
  _options?: OpenDialogOptions
): Promise<string | string[] | null> {
  console.debug("[mock] dialog.open called - returning null");
  return null;
}

/**
 * Mock save dialog - returns null (user cancelled)
 */
export async function save(_options?: SaveDialogOptions): Promise<string | null> {
  console.debug("[mock] dialog.save called - returning null");
  return null;
}

/**
 * Mock message dialog - resolves immediately
 */
export async function message(
  _message: string,
  _options?: string | MessageDialogOptions
): Promise<void> {
  console.debug("[mock] dialog.message called");
}

/**
 * Mock ask dialog - returns true (confirmed)
 */
export async function ask(
  _message: string,
  _options?: string | ConfirmDialogOptions
): Promise<boolean> {
  console.debug("[mock] dialog.ask called - returning true");
  return true;
}

/**
 * Mock confirm dialog - returns true (confirmed)
 */
export async function confirm(
  _message: string,
  _options?: string | ConfirmDialogOptions
): Promise<boolean> {
  console.debug("[mock] dialog.confirm called - returning true");
  return true;
}
