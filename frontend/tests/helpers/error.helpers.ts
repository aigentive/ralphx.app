import { Page } from "@playwright/test";

/**
 * Error modes that can be triggered for testing
 */
export type ErrorMode = "network" | "component" | "data" | "none";

/**
 * Helper to inject errors into API responses to capture error states
 */
export async function enableErrorMode(
  page: Page,
  mode: ErrorMode = "network",
  errorMessage: string = "Mock error for testing"
) {
  await page.evaluate(
    ({ mode, message }) => {
      window.__mockApiError = { mode, message };
    },
    { mode, message: errorMessage }
  );
}

/**
 * Helper to disable error mode and restore normal API behavior
 */
export async function disableErrorMode(page: Page) {
  await page.evaluate(() => {
    if (window.__mockApiError) {
      delete window.__mockApiError;
    }
  });
}

/**
 * Trigger a React component error by injecting a script
 * that throws on next render
 */
export async function triggerComponentError(
  page: Page,
  errorMessage: string = "Test component error"
) {
  await page.evaluate((message) => {
    // Force a render error
    window.__forceComponentError = message;
  }, errorMessage);
}

/**
 * Navigate to a page with error mode enabled
 */
export async function navigateWithError(
  page: Page,
  url: string,
  mode: ErrorMode = "network",
  errorMessage?: string
): Promise<void> {
  await enableErrorMode(page, mode, errorMessage);
  await page.goto(url, { waitUntil: "domcontentloaded" });
  // Wait for error to render
  await page.waitForTimeout(500);
}
