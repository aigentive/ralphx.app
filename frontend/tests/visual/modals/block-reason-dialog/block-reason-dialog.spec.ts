/**
 * Visual regression tests for BlockReasonDialog
 * Modal for capturing an optional reason when blocking a task
 */

import { test, expect } from "@playwright/test";
import { BlockReasonDialogPage } from "../../../pages/modals/block-reason-dialog.page";
import { setupApp } from "../../../fixtures/setup.fixtures";
import { openBlockDialogViaKanban } from "../../../helpers/block-reason-dialog.helpers";

test.describe("BlockReasonDialog Visual Tests", () => {
  let dialogPage: BlockReasonDialogPage;

  test.beforeEach(async ({ page }) => {
    dialogPage = new BlockReasonDialogPage(page);
    await setupApp(page);
    await page.evaluate(async () => {
      const { useProjectStore } = await import("/src/stores/projectStore");
      const { planApi } = await import("/src/api/plan");
      const planStore = (window as Window & {
        __planStore?: { getState(): { loadActivePlan(projectId: string): Promise<void> } };
      }).__planStore;

      useProjectStore.getState().selectProject("project-mock-1");
      await planApi.setActivePlan("project-mock-1", "plan-mock-2", "kanban_inline");
      await planStore?.getState().loadActivePlan("project-mock-1");
    });
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });
  });

  test("renders dialog with task title", async ({ page }) => {
    // Open the dialog via production UI flow
    await openBlockDialogViaKanban(page);

    // Verify dialog is visible
    await dialogPage.expectVisible();

    // Take snapshot
    await expect(dialogPage.dialog).toHaveScreenshot("block-reason-dialog-with-title.png");
  });

  test("renders empty reason input", async ({ page }) => {
    await openBlockDialogViaKanban(page);
    await dialogPage.expectVisible();

    // Verify input is empty
    await dialogPage.expectReasonValue("");

    // Take snapshot
    await expect(dialogPage.reasonInput).toHaveScreenshot("empty-reason-input.png");
  });

  test("shows entered reason text", async ({ page }) => {
    await openBlockDialogViaKanban(page);
    await dialogPage.expectVisible();

    // Enter reason
    const reason = "Waiting for API endpoint to be implemented";
    await dialogPage.enterReason(reason);

    // Verify value
    await dialogPage.expectReasonValue(reason);

    // Take snapshot
    await expect(dialogPage.dialog).toHaveScreenshot("dialog-with-reason-text.png");
  });

  test("renders cancel button", async ({ page }) => {
    await openBlockDialogViaKanban(page);
    await dialogPage.expectVisible();

    // Verify button is visible
    await expect(dialogPage.cancelButton).toBeVisible();

    // Take snapshot
    await expect(dialogPage.cancelButton).toHaveScreenshot("cancel-button.png");
  });

  test("renders confirm button with block icon", async ({ page }) => {
    await openBlockDialogViaKanban(page);
    await dialogPage.expectVisible();

    // Verify button is visible
    await expect(dialogPage.confirmButton).toBeVisible();

    // Take snapshot
    await expect(dialogPage.confirmButton).toHaveScreenshot("confirm-button.png");
  });

  test("dialog closes on cancel", async ({ page }) => {
    await openBlockDialogViaKanban(page);
    await dialogPage.expectVisible();

    // Click cancel
    await dialogPage.clickCancel();

    // Verify dialog is hidden
    await dialogPage.expectHidden();
  });

  test("dialog closes on confirm", async ({ page }) => {
    await openBlockDialogViaKanban(page);
    await dialogPage.expectVisible();

    // Enter reason
    await dialogPage.enterReason("Test reason");

    // Click confirm
    await dialogPage.clickConfirm();

    // Verify dialog is hidden
    await dialogPage.expectHidden();
  });

  test("supports keyboard shortcut (Cmd+Enter)", async ({ page }) => {
    await openBlockDialogViaKanban(page);
    await dialogPage.expectVisible();

    // Enter reason
    await dialogPage.enterReason("Keyboard test");

    // Use keyboard shortcut
    await dialogPage.confirmWithKeyboard();

    // Verify dialog is hidden
    await dialogPage.expectHidden();
  });
});
