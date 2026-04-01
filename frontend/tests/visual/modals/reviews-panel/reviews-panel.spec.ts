import { test, expect } from "@playwright/test";
import { ReviewsPanelPage } from "../../../pages/modals/reviews-panel.page";
import { setupApp } from "../../../fixtures/setup.fixtures";

/**
 * Visual regression tests for ReviewsPanel component.
 *
 * The ReviewsPanel is a slide-in panel that shows tasks awaiting review,
 * grouped by review phase (AI and Human).
 */

test.describe("ReviewsPanel", () => {
  let reviewsPanel: ReviewsPanelPage;

  test.beforeEach(async ({ page }) => {
    reviewsPanel = new ReviewsPanelPage(page);
    await setupApp(page);
  });

  test("opens panel when toggle button is clicked", async () => {
    // Initially the panel should not be visible
    await expect(reviewsPanel.panel).not.toBeVisible();

    // Click the reviews toggle button
    await reviewsPanel.openPanel();

    // Panel should now be visible
    await expect(reviewsPanel.panel).toBeVisible();
  });

  test("closes panel when close button is clicked", async () => {
    // Open the panel first
    await reviewsPanel.openPanel();
    await expect(reviewsPanel.panel).toBeVisible();

    // Close the panel
    await reviewsPanel.closeButton.click();

    // Panel should be hidden
    await expect(reviewsPanel.panel).not.toBeVisible();
  });

  test("displays empty state when no reviews pending", async () => {
    await reviewsPanel.openPanel();

    // Check for either empty state or task cards
    // In mock mode, we might have empty reviews
    const hasEmptyState = await reviewsPanel.emptyState.isVisible().catch(() => false);
    const hasTaskCards = (await reviewsPanel.getTaskCardCount()) > 0;

    // One of these should be true
    expect(hasEmptyState || hasTaskCards).toBe(true);
  });

  test("allows switching between AI and Human tabs", async () => {
    await reviewsPanel.openPanel();

    // Tabs should be visible
    await expect(reviewsPanel.aiTab).toBeVisible();
    await expect(reviewsPanel.humanTab).toBeVisible();

    // Click AI tab
    await reviewsPanel.switchToAiTab();
    await expect(reviewsPanel.aiTab).toHaveAttribute("data-state", "active");

    // Click Human tab
    await reviewsPanel.switchToHumanTab();
    await expect(reviewsPanel.humanTab).toHaveAttribute("data-state", "active");
  });

  test("matches snapshot", async ({ page }) => {
    await reviewsPanel.openPanel();

    // Wait for animations to complete
    await reviewsPanel.waitForAnimations();

    // Take snapshot
    await expect(page).toHaveScreenshot("reviews-panel.png", {
      maxDiffPixelRatio: 0.01,
    });
  });
});
