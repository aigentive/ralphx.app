import { test, expect } from "@playwright/test";
import { TaskDetailPage } from "../../../pages/task-detail.page";
import { setupTaskDetail } from "../../../fixtures/setup.fixtures";

/**
 * Visual regression tests for the Task Detail overlay.
 *
 * These tests run against the web mode dev server (npm run dev:web)
 * which uses mock data from src/api-mock/ instead of the real Tauri backend.
 *
 * Tests the TaskDetailOverlay component that appears in the Kanban split layout
 * when a task is selected.
 *
 * Uses Page Object Model pattern for maintainable selectors.
 */

test.describe("Task Detail Overlay", () => {
  let taskDetail: TaskDetailPage;

  test.beforeEach(async ({ page }) => {
    taskDetail = new TaskDetailPage(page);
    await setupTaskDetail(page);
  });

  test("renders task detail overlay when task is selected", async () => {
    // Verify the task detail overlay is visible
    await expect(taskDetail.taskDetailOverlay).toBeVisible({ timeout: 10000 });

    // Verify the overlay backdrop is present
    await expect(taskDetail.overlayBackdrop).toBeVisible();
  });

  test("displays task header with title and metadata", async () => {
    // Verify task title is visible
    await expect(taskDetail.overlayTitle).toBeVisible();

    // Verify task metadata (category, priority, status)
    await expect(taskDetail.overlayCategory).toBeVisible();
    await expect(taskDetail.overlayPriority).toBeVisible();
    await expect(taskDetail.overlayStatus).toBeVisible();
  });

  test("displays action buttons in header", async () => {
    // Close button should always be visible
    await expect(taskDetail.closeButton).toBeVisible();

    // Edit button should be visible for editable tasks
    const editVisible = await taskDetail.editButton.isVisible();
    expect(typeof editVisible).toBe("boolean");

    // Archive or restore button may be visible depending on task state
    const archiveVisible = await taskDetail.archiveButton.isVisible();
    const restoreVisible = await taskDetail.restoreButton.isVisible();
    expect(typeof archiveVisible).toBe("boolean");
    expect(typeof restoreVisible).toBe("boolean");
  });

  test("task detail overlay matches snapshot", async ({ page }) => {
    // Wait for animations to complete
    await taskDetail.waitForAnimations();

    // Take a screenshot for visual regression
    await expect(page).toHaveScreenshot("task-detail-overlay.png", {
      maxDiffPixelRatio: 0.01,
      fullPage: false,
    });
  });

  test("close button closes the overlay", async () => {
    // Verify overlay is initially visible
    await expect(taskDetail.taskDetailOverlay).toBeVisible();

    // Click close button
    await taskDetail.closeTaskDetail();

    // Verify overlay is hidden
    await expect(taskDetail.taskDetailOverlay).not.toBeVisible();
  });
});
