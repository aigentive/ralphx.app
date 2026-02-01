/**
 * Test Helpers: ScreenshotGallery
 *
 * Utilities for testing ScreenshotGallery with mock data in visual tests.
 */

import { Page } from "@playwright/test";

/**
 * Navigate to the screenshot gallery test page
 *
 * This navigates to a special test page route that renders ScreenshotGallery
 * with mock data. The test page is registered in App.tsx for web mode testing.
 */
export async function openScreenshotGalleryTestPage(
  page: Page,
  scenario: "default" | "empty" | "twoColumns" | "fourColumns" = "default"
): Promise<void> {
  // Navigate to base URL
  await page.goto(`http://localhost:5173/?test=screenshot-gallery&scenario=${scenario}`);
  await page.waitForLoadState("networkidle");

  // Wait for gallery to render
  await page.waitForTimeout(500);
}
