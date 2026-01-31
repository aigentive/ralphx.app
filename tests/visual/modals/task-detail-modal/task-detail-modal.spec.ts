import { test, expect } from "@playwright/test";
import { TaskDetailModalPage } from "../../../pages/modals/task-detail-modal.page";
import { setupApp } from "../../../fixtures/setup.fixtures";
import { openTaskDetailModal, closeTaskDetailModal } from "../../../helpers/task-detail.helpers";
import {
  createMockTask,
  createArchivedTask,
  createTaskWithContext,
  createExecutingTask,
} from "../../../fixtures/tasks.fixtures";

/**
 * Visual regression tests for TaskDetailModal component.
 *
 * TaskDetailModal is a premium modal dialog for viewing and editing task details.
 * It's opened programmatically via uiStore.openModal("task-detail", { task }).
 */

test.describe("TaskDetailModal", () => {
  let modal: TaskDetailModalPage;

  test.beforeEach(async ({ page }) => {
    modal = new TaskDetailModalPage(page);
    await setupApp(page);
  });

  test("opens modal with basic task", async ({ page }) => {
    const task = createMockTask();
    await openTaskDetailModal(page, task);

    // Modal should be visible
    await expect(modal.modal).toBeVisible();

    // Verify basic content
    await expect(modal.title).toHaveText(task.title);
    await expect(modal.category).toContainText(task.category);
  });

  test("displays task description", async ({ page }) => {
    const task = createMockTask({
      description: "Test task description with important details",
    });
    await openTaskDetailModal(page, task);

    await expect(modal.description).toBeVisible();
    await expect(modal.description).toContainText("Test task description");
  });

  test("shows archived badge for archived tasks", async ({ page }) => {
    const task = createArchivedTask();
    await openTaskDetailModal(page, task);

    await expect(modal.archivedBadge).toBeVisible();
  });

  test("shows restore button for archived tasks", async ({ page }) => {
    const task = createArchivedTask();
    await openTaskDetailModal(page, task);

    await expect(modal.restoreButton).toBeVisible();
    await expect(modal.archiveButton).not.toBeVisible();
  });

  test("shows archive button for non-archived tasks", async ({ page }) => {
    const task = createMockTask();
    await openTaskDetailModal(page, task);

    await expect(modal.archiveButton).toBeVisible();
    await expect(modal.restoreButton).not.toBeVisible();
  });

  test("shows edit button for editable tasks", async ({ page }) => {
    const task = createMockTask({ internalStatus: "ready" });
    await openTaskDetailModal(page, task);

    await expect(modal.editButton).toBeVisible();
  });

  test("hides edit button for system-controlled tasks", async ({ page }) => {
    const task = createExecutingTask();
    await openTaskDetailModal(page, task);

    // Edit button should not be visible for executing tasks
    await expect(modal.editButton).not.toBeVisible();
  });

  test("shows context button for tasks with source", async ({ page }) => {
    const task = createTaskWithContext();
    await openTaskDetailModal(page, task);

    await expect(modal.viewContextButton).toBeVisible();
  });

  test("hides context button for tasks without source", async ({ page }) => {
    const task = createMockTask({
      sourceProposalId: null,
      planArtifactId: null,
    });
    await openTaskDetailModal(page, task);

    await expect(modal.viewContextButton).not.toBeVisible();
  });

  test("closes when close button is clicked", async ({ page }) => {
    const task = createMockTask();
    await openTaskDetailModal(page, task);

    await expect(modal.modal).toBeVisible();

    await closeTaskDetailModal(page);

    await expect(modal.modal).not.toBeVisible();
  });

  test("matches snapshot - basic task", async ({ page }) => {
    const task = createMockTask();
    await openTaskDetailModal(page, task);

    // Wait for content to load
    await page.waitForTimeout(500);

    await expect(page).toHaveScreenshot("task-detail-modal-basic.png", {
      maxDiffPixelRatio: 0.01,
    });
  });

  test("matches snapshot - archived task", async ({ page }) => {
    const task = createArchivedTask();
    await openTaskDetailModal(page, task);

    await page.waitForTimeout(500);

    await expect(page).toHaveScreenshot("task-detail-modal-archived.png", {
      maxDiffPixelRatio: 0.01,
    });
  });

  test("matches snapshot - task with context", async ({ page }) => {
    const task = createTaskWithContext();
    await openTaskDetailModal(page, task);

    await page.waitForTimeout(500);

    await expect(page).toHaveScreenshot("task-detail-modal-context.png", {
      maxDiffPixelRatio: 0.01,
    });
  });

  test("matches snapshot - system-controlled task", async ({ page }) => {
    const task = createExecutingTask();
    await openTaskDetailModal(page, task);

    await page.waitForTimeout(500);

    await expect(page).toHaveScreenshot("task-detail-modal-executing.png", {
      maxDiffPixelRatio: 0.01,
    });
  });
});
