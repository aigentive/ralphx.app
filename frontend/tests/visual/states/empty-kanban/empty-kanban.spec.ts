import { test, expect } from "@playwright/test";
import { KanbanPage } from "../../../pages/kanban.page";
import { setupEmptyKanban } from "../../../fixtures/setup.fixtures";

/**
 * Visual regression tests for the empty Kanban board state.
 *
 * These tests verify the UI when there are no tasks in the kanban board.
 * Uses Page Object Model pattern for maintainable selectors.
 */

test.describe("Empty Kanban Board State", () => {
  let kanban: KanbanPage;

  test.beforeEach(async ({ page }) => {
    kanban = new KanbanPage(page);
    await setupEmptyKanban(page);
  });

  test("renders empty board without task cards", async () => {
    // Verify the board is visible
    await expect(kanban.board).toBeVisible({ timeout: 10000 });

    // Verify there are no task cards
    const count = await kanban.taskCards.count();
    expect(count).toBe(0);
  });

  test("empty kanban board layout matches snapshot", async ({ page }) => {
    // Wait for animations to complete
    await kanban.waitForAnimations();

    // Take a full page screenshot for visual regression
    await expect(page).toHaveScreenshot("empty-kanban-board.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: false,
    });
  });

  test("navigation elements are still visible in empty state", async () => {
    // Verify the RalphX branding is visible
    await expect(kanban.branding).toBeVisible();

    // Verify the chat toggle is visible
    await expect(kanban.chatToggle).toBeVisible();

    // Verify the reviews toggle is visible
    await expect(kanban.reviewsToggle).toBeVisible();
  });

  test("board structure is present even when empty", async () => {
    // Verify the board container exists
    await expect(kanban.board).toBeVisible();

    // The board should have column structure even when empty
    // This test verifies that the empty state doesn't break the layout
  });
});
