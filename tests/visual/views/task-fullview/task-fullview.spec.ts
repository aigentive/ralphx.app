import { test, expect } from "@playwright/test";
import { TaskFullViewPage } from "../../../pages/views/task-fullview.page";
import { openTaskFullView } from "../../../helpers/task-fullview.helpers";

/**
 * Visual regression tests for TaskFullView component
 *
 * Uses mock data from src/api-mock/store.ts:
 * - task-mock-1: backlog
 * - task-mock-2: ready
 * - task-mock-3: blocked
 * - task-mock-4: executing (shows footer controls)
 * - task-mock-5: pending_review (shows footer controls)
 */

test.describe("TaskFullView Visual Tests", () => {
  let taskFullViewPage: TaskFullViewPage;

  test.beforeEach(async ({ page }) => {
    taskFullViewPage = new TaskFullViewPage(page);
    await page.goto("http://localhost:5173");
    await page.waitForLoadState("networkidle");
    // Wait for app to fully load
    await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });
  });

  test("renders TaskFullView with ready task", async ({ page }) => {
    await openTaskFullView(page, "task-mock-2"); // ready status
    await taskFullViewPage.waitForVisible();

    await expect(taskFullViewPage.container).toBeVisible();
    await expect(taskFullViewPage.header).toBeVisible();
    await expect(taskFullViewPage.leftPanel).toBeVisible();
    await expect(taskFullViewPage.rightPanel).toBeVisible();

    await expect(page).toHaveScreenshot("task-fullview-ready.png");
  });

  test("renders TaskFullView with executing task", async ({ page }) => {
    await openTaskFullView(page, "task-mock-4"); // executing status
    await taskFullViewPage.waitForVisible();

    await expect(taskFullViewPage.container).toBeVisible();
    // Executing status should show footer controls
    await expect(taskFullViewPage.footer).toBeVisible();
    await expect(taskFullViewPage.pauseButton).toBeVisible();
    await expect(taskFullViewPage.stopButton).toBeVisible();

    await expect(page).toHaveScreenshot("task-fullview-executing.png");
  });

  test("renders TaskFullView with pending review task", async ({ page }) => {
    await openTaskFullView(page, "task-mock-5"); // pending_review status
    await taskFullViewPage.waitForVisible();

    await expect(taskFullViewPage.container).toBeVisible();
    // Pending review is not an execution state, footer should be hidden
    const hasControls = await taskFullViewPage.hasExecutionControls();
    expect(hasControls).toBe(false);

    await expect(page).toHaveScreenshot("task-fullview-pending-review.png");
  });

  test("displays task header with title, status, and priority", async ({ page }) => {
    await openTaskFullView(page, "task-mock-2");
    await taskFullViewPage.waitForVisible();

    await expect(taskFullViewPage.title).toBeVisible();
    await expect(taskFullViewPage.status).toBeVisible();
    await expect(taskFullViewPage.priority).toBeVisible();

    const title = await taskFullViewPage.getTitle();
    expect(title).toBeTruthy();

    await expect(page).toHaveScreenshot("task-fullview-header.png");
  });

  test("displays header action buttons", async ({ page }) => {
    await openTaskFullView(page, "task-mock-2");
    await taskFullViewPage.waitForVisible();

    await expect(taskFullViewPage.backButton).toBeVisible();
    await expect(taskFullViewPage.editButton).toBeVisible();
    await expect(taskFullViewPage.archiveButton).toBeVisible();
    await expect(taskFullViewPage.closeButton).toBeVisible();

    await expect(page).toHaveScreenshot("task-fullview-actions.png");
  });

  test("shows split layout with resizable panels", async ({ page }) => {
    await openTaskFullView(page, "task-mock-2");
    await taskFullViewPage.waitForVisible();

    await expect(taskFullViewPage.leftPanel).toBeVisible();
    await expect(taskFullViewPage.rightPanel).toBeVisible();
    await expect(taskFullViewPage.dragHandle).toBeVisible();

    await expect(page).toHaveScreenshot("task-fullview-split-layout.png");
  });

  test("closes via back button", async ({ page }) => {
    await openTaskFullView(page, "task-mock-2");
    await taskFullViewPage.waitForVisible();

    await taskFullViewPage.clickBack();
    await taskFullViewPage.waitForHidden();

    await expect(taskFullViewPage.container).not.toBeVisible();
  });

  test("closes via X button", async ({ page }) => {
    await openTaskFullView(page, "task-mock-2");
    await taskFullViewPage.waitForVisible();

    await taskFullViewPage.clickClose();
    await taskFullViewPage.waitForHidden();

    await expect(taskFullViewPage.container).not.toBeVisible();
  });

  test("shows footer execution controls for executing status", async ({ page }) => {
    await openTaskFullView(page, "task-mock-4"); // executing
    await taskFullViewPage.waitForVisible();

    const hasControls = await taskFullViewPage.hasExecutionControls();
    expect(hasControls).toBe(true);

    await expect(page).toHaveScreenshot("task-fullview-execution-controls.png");
  });

  test("hides footer for non-executing status", async ({ page }) => {
    await openTaskFullView(page, "task-mock-2"); // ready status
    await taskFullViewPage.waitForVisible();

    const hasControls = await taskFullViewPage.hasExecutionControls();
    expect(hasControls).toBe(false);

    await expect(page).toHaveScreenshot("task-fullview-no-footer.png");
  });
});
