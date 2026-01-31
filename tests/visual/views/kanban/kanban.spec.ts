import { test, expect } from "@playwright/test";

/**
 * Visual regression tests for the Kanban board.
 *
 * These tests run against the web mode dev server (npm run dev:web)
 * which uses mock data from src/api-mock/ instead of the real Tauri backend.
 */

test.describe("Kanban Board", () => {
  test("renders task cards with mock data", async ({ page }) => {
    // Navigate to the app root
    await page.goto("/");

    // In web mode with mock data, the app should:
    // 1. Have mock projects loaded
    // 2. Auto-select the first project
    // 3. Show the kanban view with task cards

    // Wait for the app to load and hydrate
    // The header should be visible indicating the app has loaded
    await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });

    // Check that task cards are rendered
    // Task cards have data-testid="task-card-{id}" pattern
    // We use a CSS selector to match any task card
    const taskCards = page.locator('[data-testid^="task-card-"]');

    // Wait for at least one task card to appear
    // This confirms mock data is being used and the kanban board is rendering
    await expect(taskCards.first()).toBeVisible({ timeout: 10000 });

    // Verify we have multiple task cards (mock data creates several)
    const count = await taskCards.count();
    expect(count).toBeGreaterThan(0);
  });

  test("kanban board layout matches snapshot", async ({ page }) => {
    await page.goto("/");

    // Wait for the app to fully load
    await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });
    await page.waitForSelector('[data-testid^="task-card-"]', { timeout: 10000 });

    // Give time for animations to complete
    await page.waitForTimeout(500);

    // Take a full page screenshot for visual regression
    // The snapshot will be stored in tests/visual/snapshots/
    await expect(page).toHaveScreenshot("kanban-board.png", {
      // Allow slight differences due to font rendering variations
      maxDiffPixelRatio: 0.01,
      // Capture full page
      fullPage: false,
    });
  });

  test("navigation tabs are visible", async ({ page }) => {
    await page.goto("/");

    // Wait for header to load
    await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });

    // Verify the RalphX branding is visible
    const branding = page.locator("text=RalphX");
    await expect(branding).toBeVisible();

    // Verify the chat toggle is visible
    const chatToggle = page.locator('[data-testid="chat-toggle"]');
    await expect(chatToggle).toBeVisible();

    // Verify the reviews toggle is visible
    const reviewsToggle = page.locator('[data-testid="reviews-toggle"]');
    await expect(reviewsToggle).toBeVisible();
  });
});
