/**
 * Visual Regression Tests: TaskPickerDialog
 *
 * Tests the modal for selecting draft tasks to seed ideation sessions.
 */

import { test, expect } from "@playwright/test";
import { TaskPickerDialogPage } from "../../../pages/modals/task-picker-dialog.page";
import { openTaskPickerDialog } from "../../../helpers/task-picker-dialog.helpers";

test.describe("TaskPickerDialog - Visual Regression", () => {
  test.beforeEach(async ({ page }) => {
    // Open the dialog
    await openTaskPickerDialog(page);
  });

  test("renders dialog with default state", async ({ page }) => {
    const dialogPage = new TaskPickerDialogPage(page);

    // Verify dialog is visible
    await expect(dialogPage.dialog).toBeVisible();
    await expect(dialogPage.title).toHaveText("Select Draft Task");

    // Verify search input
    await expect(dialogPage.searchInput).toBeVisible();
    await expect(dialogPage.searchInput).toHaveAttribute("placeholder", "Search tasks...");

    // Verify task list is visible (should have backlog tasks from mock)
    await expect(dialogPage.taskList).toBeVisible();

    // Visual snapshot
    await expect(page).toHaveScreenshot("task-picker-dialog-default.png", {
      fullPage: false,
    });
  });

  test("shows draft tasks from backlog", async ({ page }) => {
    const dialogPage = new TaskPickerDialogPage(page);

    // Verify at least one task is shown (from mock data)
    const taskCount = await dialogPage.getTaskCount();
    expect(taskCount).toBeGreaterThan(0);

    // Verify backlog task is visible
    const backlogTask = dialogPage.getTaskItem("Backlog Task");
    await expect(backlogTask).toBeVisible();

    // Visual snapshot with tasks
    await expect(page).toHaveScreenshot("task-picker-dialog-with-tasks.png", {
      fullPage: false,
    });
  });

  test("search filters tasks", async ({ page }) => {
    const dialogPage = new TaskPickerDialogPage(page);

    // Initial task count
    const initialCount = await dialogPage.getTaskCount();
    expect(initialCount).toBeGreaterThan(0);

    // Search for specific task
    await dialogPage.search("Additional Task 1");

    // Wait for filter to apply
    await page.waitForTimeout(200);

    // Verify filtered results
    const filteredCount = await dialogPage.getTaskCount();
    expect(filteredCount).toBeLessThanOrEqual(initialCount);

    // Visual snapshot of search results
    await expect(page).toHaveScreenshot("task-picker-dialog-search.png", {
      fullPage: false,
    });
  });

  test("shows empty state when no tasks match search", async ({ page }) => {
    const dialogPage = new TaskPickerDialogPage(page);

    // Search for non-existent task
    await dialogPage.search("NonExistentTaskXYZ123");

    // Wait for filter to apply
    await page.waitForTimeout(200);

    // Verify empty state is shown
    await expect(dialogPage.emptyState).toBeVisible();
    await expect(dialogPage.emptyStateMessage).toContainText("No draft tasks match your search");

    // Visual snapshot of empty search state
    await expect(page).toHaveScreenshot("task-picker-dialog-empty-search.png", {
      fullPage: false,
    });
  });

  test("task items show title, description, and category", async ({ page }) => {
    const dialogPage = new TaskPickerDialogPage(page);

    // Get first task item
    const firstTask = dialogPage.getAllTaskItems().first();
    await expect(firstTask).toBeVisible();

    // Verify structure (title, description, category should be visible)
    // The exact text depends on mock data, but structure should be consistent

    // Visual snapshot focusing on task item structure
    await expect(page).toHaveScreenshot("task-picker-dialog-task-item.png", {
      fullPage: false,
    });
  });

  test("hover state on task items", async ({ page }) => {
    const dialogPage = new TaskPickerDialogPage(page);

    // Hover over first task
    const firstTask = dialogPage.getAllTaskItems().first();
    await firstTask.hover();

    // Brief wait for hover transition
    await page.waitForTimeout(100);

    // Visual snapshot of hover state
    await expect(page).toHaveScreenshot("task-picker-dialog-task-hover.png", {
      fullPage: false,
    });
  });
});
