import { test, expect } from "@playwright/test";
import { ActivityPage } from "../../../pages/activity.page";
import { setupActivity } from "../../../fixtures/setup.fixtures";

/**
 * Visual regression tests for the Activity view.
 *
 * These tests run against the web mode dev server (cd frontend && npm run dev:web)
 * which uses mock data from frontend/src/api-mock/ instead of the real Tauri backend.
 *
 * Uses Page Object Model pattern for maintainable selectors.
 */

test.describe("Activity View", () => {
  let activity: ActivityPage;

  test.beforeEach(async ({ page }) => {
    activity = new ActivityPage(page);
    await setupActivity(page);
  });

  test("renders activity view after navigation", async () => {
    await expect(activity.view).toBeVisible();
  });

  test("displays header with clear button", async () => {
    await expect(activity.clearButton).toBeVisible();
  });

  test("displays search bar and filter controls", async () => {
    await expect(activity.searchBar).toBeVisible();
    await expect(activity.allFilterTab).toBeVisible();
    await expect(activity.thinkingFilterTab).toBeVisible();
    await expect(activity.toolCallsFilterTab).toBeVisible();
  });

  test("displays messages container", async () => {
    await expect(activity.messagesContainer).toBeVisible();
  });

  test("activity view layout matches snapshot", async ({ page }) => {
    // Wait for animations to complete
    await activity.waitForAnimations();

    // Take a full page screenshot for visual regression
    await expect(page).toHaveScreenshot("activity-view.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: false,
    });
  });

  test("navigation shows activity tab as active", async () => {
    // Verify the activity nav button is marked as the active route without
    // pinning the test to an exact accent RGB value.
    await expect(activity.navActivity).toBeVisible();
    await expect(activity.navActivity).toHaveAttribute("aria-current", "page");
  });

  test("empty state is shown when no messages exist", async () => {
    const hasMessages = await activity.hasMessages();
    if (!hasMessages) {
      await expect(activity.emptyState).toBeVisible();
    }
  });

  test("clear button is disabled when no messages exist", async () => {
    const hasMessages = await activity.hasMessages();
    if (!hasMessages) {
      await expect(activity.clearButton).toBeDisabled();
    }
  });
});
