/**
 * Test helpers for WelcomeScreen component
 */

import type { Page } from "@playwright/test";

/**
 * Open the WelcomeScreen overlay in web mode
 * Uses window.__uiStore to trigger the overlay state
 */
export async function openWelcomeScreen(page: Page): Promise<void> {
  await page.evaluate(() => {
    if (window.__uiStore) {
      window.__uiStore.getState().openWelcomeOverlay();
    }
  });

  // Wait for the welcome screen to appear
  await page.waitForSelector('[data-testid="welcome-screen"]', { timeout: 5000 });
}

/**
 * Close the WelcomeScreen overlay
 */
export async function closeWelcomeScreen(page: Page): Promise<void> {
  await page.evaluate(() => {
    if (window.__uiStore) {
      window.__uiStore.getState().closeWelcomeOverlay();
    }
  });

  // Wait for the welcome screen to disappear
  await page.waitForSelector('[data-testid="welcome-screen"]', { state: "hidden", timeout: 5000 });
}
