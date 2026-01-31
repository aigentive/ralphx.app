import { test, expect } from "@playwright/test";
import { IdeationPage } from "../../../pages/ideation.page";
import { setupIdeation } from "../../../fixtures/setup.fixtures";

/**
 * Visual regression tests for the Ideation view.
 *
 * These tests run against the web mode dev server (npm run dev:web)
 * which uses mock data from src/api-mock/ instead of the real Tauri backend.
 *
 * Uses Page Object Model pattern for maintainable selectors.
 */

test.describe("Ideation View", () => {
  let ideation: IdeationPage;

  test.beforeEach(async ({ page }) => {
    ideation = new IdeationPage(page);
    await setupIdeation(page);
  });

  test("renders ideation view after navigation", async () => {
    await expect(ideation.view).toBeVisible();
  });

  test("displays session browser sidebar", async () => {
    await expect(ideation.sessionBrowser).toBeVisible();
  });

  test("displays header with session title when session is active", async () => {
    // Check if there's an active session (mock data provides one)
    const hasSession = await ideation.hasActiveSession();
    if (hasSession) {
      await expect(ideation.header).toBeVisible();
    }
  });

  test("ideation view layout matches snapshot", async ({ page }) => {
    // Wait for animations to complete
    await ideation.waitForAnimations();

    // Take a full page screenshot for visual regression
    await expect(page).toHaveScreenshot("ideation-view.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: false,
    });
  });

  test("navigation shows ideation tab as active", async () => {
    // Verify the ideation nav button is highlighted/active
    await expect(ideation.navIdeation).toBeVisible();
    // The active state shows the orange accent color
    await expect(ideation.navIdeation).toHaveCSS("color", "rgb(255, 107, 53)");
  });
});
