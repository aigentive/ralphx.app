/**
 * Mock implementation of @tauri-apps/plugin-updater for web mode
 *
 * Update checking is not available in browser mode.
 * Returns null (no update available) to prevent UI prompts.
 */

export interface DownloadProgress {
  event: "Started" | "Progress" | "Finished";
  data: {
    contentLength?: number;
    chunkLength: number;
  };
}

export interface Update {
  /** New version string */
  version: string;
  /** Current installed version */
  currentVersion: string;
  /** Release notes/changelog */
  body?: string;
  /** Release date */
  date?: string;
  /** Download and install the update with progress callback */
  downloadAndInstall: (
    onProgress?: (progress: DownloadProgress) => void
  ) => Promise<void>;
}

/**
 * Mock check for updates - returns null (no update available)
 */
export async function check(): Promise<Update | null> {
  console.debug("[mock] updater.check called - returning null (no update)");
  return null;
}
