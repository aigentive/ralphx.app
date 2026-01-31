import { test, expect } from "@playwright/test";
import { KanbanPage } from "../../../pages/kanban.page";
import { setupKanban } from "../../../fixtures/setup.fixtures";

/**
 * Visual regression tests for the Kanban board.
 *
 * These tests run against the web mode dev server (npm run dev:web)
 * which uses mock data from src/api-mock/ instead of the real Tauri backend.
 *
 * Uses Page Object Model pattern for maintainable selectors.
 */

test.describe("Kanban Board", () => {
  let kanban: KanbanPage;

  test.beforeEach(async ({ page }) => {
    kanban = new KanbanPage(page);
    await setupKanban(page);
  });

  test("renders task cards with mock data", async () => {
    // Verify we have task cards rendered with mock data
    await expect(kanban.taskCards.first()).toBeVisible({ timeout: 10000 });

    // Verify we have multiple task cards
    const count = await kanban.getTaskCount();
    expect(count).toBeGreaterThan(0);
  });

  test("kanban board layout matches snapshot", async ({ page }) => {
    // Wait for animations to complete
    await kanban.waitForAnimations();

    // Take a full page screenshot for visual regression
    await expect(page).toHaveScreenshot("kanban-board.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: false,
    });
  });

  test("navigation tabs are visible", async () => {
    // Verify the RalphX branding is visible
    await expect(kanban.branding).toBeVisible();

    // Verify the chat toggle is visible
    await expect(kanban.chatToggle).toBeVisible();

    // Verify the reviews toggle is visible
    await expect(kanban.reviewsToggle).toBeVisible();
  });
});
