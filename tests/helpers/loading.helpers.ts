import { Page } from "@playwright/test";

/**
 * Helper to inject a delay into API responses to capture loading states
 */
export async function enableSlowMode(page: Page, delayMs: number = 2000) {
  await page.evaluate((delay) => {
    if (!window.__mockApiDelay) {
      window.__mockApiDelay = delay;
    }
  }, delayMs);
}

/**
 * Helper to disable slow mode and restore normal API speed
 */
export async function disableSlowMode(page: Page) {
  await page.evaluate(() => {
    if (window.__mockApiDelay) {
      delete window.__mockApiDelay;
    }
  });
}

/**
 * Navigate to a page and immediately pause to capture loading state
 */
export async function captureLoadingState(
  page: Page,
  url: string,
  delayMs: number = 3000
): Promise<void> {
  await enableSlowMode(page, delayMs);
  await page.goto(url);
  // Wait a bit for the skeleton to render
  await page.waitForTimeout(200);
}
