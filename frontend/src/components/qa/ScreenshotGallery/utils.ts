/**
 * Utility functions for ScreenshotGallery component
 */

import type { QAStepResult } from "@/types/qa";
import type { Screenshot } from "./ScreenshotGallery";

/**
 * Extract filename from path
 */
function getFilename(path: string): string {
  return path.split("/").pop() || path;
}

/**
 * Convert an array of screenshot paths to Screenshot objects
 * Useful for integration with TaskDetailQAPanel
 */
export function pathsToScreenshots(
  paths: string[],
  stepResults?: Map<string, QAStepResult>
): Screenshot[] {
  return paths.map((path, index) => {
    const filename = getFilename(path);
    // Try to match with step result based on filename
    const matchingResult = stepResults
      ? Array.from(stepResults.values()).find((r) => r.screenshot === path)
      : undefined;

    return {
      id: `screenshot-${index}`,
      path,
      label: matchingResult?.step_id || filename,
      stepResult: matchingResult,
    };
  });
}
