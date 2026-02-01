/**
 * Visual regression tests for MergeWorkflowDialog component
 */

import { test, expect } from "@playwright/test";
import { MergeWorkflowDialogPage } from "../../../pages/modals/merge-workflow-dialog.page";
import {
  openMergeWorkflowDialog,
  setMergeWorkflowProcessing,
  setMergeWorkflowError,
} from "../../../helpers/merge-workflow-dialog.helpers";

test.describe("MergeWorkflowDialog Visual Tests", () => {
  let dialogPage: MergeWorkflowDialogPage;

  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:5173");
    dialogPage = new MergeWorkflowDialogPage(page);
  });

  test("should render dialog with all elements", async ({ page }) => {
    // Open dialog with default state
    await openMergeWorkflowDialog(page);

    // Verify dialog is visible
    await expect(dialogPage.dialog).toBeVisible();

    // Verify header
    await expect(dialogPage.title).toContainText("Project Complete");
    await expect(dialogPage.title).toContainText("Test Project");

    // Verify summary
    await expect(dialogPage.commitCount).toContainText("5 commits");
    await expect(dialogPage.branchName).toContainText("ralphx/test-feature");

    // Verify merge option is selected by default
    expect(await dialogPage.isOptionSelected("merge")).toBe(true);

    // Verify buttons
    await expect(dialogPage.cancelButton).toBeVisible();
    await expect(dialogPage.confirmButton).toBeVisible();
    await expect(dialogPage.confirmButton).toContainText("Continue");

    // Take screenshot
    await expect(page).toHaveScreenshot("merge-workflow-dialog-initial.png", {
      fullPage: true,
    });
  });

  test("should display all workflow options", async ({ page }) => {
    await openMergeWorkflowDialog(page);

    // Verify all 5 options are visible
    const options = ["merge", "rebase", "create_pr", "keep_worktree", "discard"] as const;

    for (const option of options) {
      const optionElement = page.getByTestId(`merge-option-${option}`);
      await expect(optionElement).toBeVisible();
    }

    await expect(page).toHaveScreenshot("merge-workflow-dialog-options.png", {
      fullPage: true,
    });
  });

  test("should select rebase option", async ({ page }) => {
    await openMergeWorkflowDialog(page);

    // Select rebase
    await dialogPage.selectOption("rebase");

    // Verify selection
    expect(await dialogPage.isOptionSelected("rebase")).toBe(true);
    expect(await dialogPage.isOptionSelected("merge")).toBe(false);

    await expect(page).toHaveScreenshot("merge-workflow-dialog-rebase-selected.png", {
      fullPage: true,
    });
  });

  test("should select create PR option", async ({ page }) => {
    await openMergeWorkflowDialog(page);

    // Select create PR
    await dialogPage.selectOption("create_pr");

    // Verify selection
    expect(await dialogPage.isOptionSelected("create_pr")).toBe(true);

    await expect(page).toHaveScreenshot("merge-workflow-dialog-create-pr-selected.png", {
      fullPage: true,
    });
  });

  test("should select keep worktree option", async ({ page }) => {
    await openMergeWorkflowDialog(page);

    // Select keep worktree
    await dialogPage.selectOption("keep_worktree");

    // Verify selection
    expect(await dialogPage.isOptionSelected("keep_worktree")).toBe(true);

    await expect(page).toHaveScreenshot("merge-workflow-dialog-keep-worktree-selected.png", {
      fullPage: true,
    });
  });

  test("should show discard confirmation", async ({ page }) => {
    await openMergeWorkflowDialog(page);

    // Select discard option
    await dialogPage.selectOption("discard");

    // Verify discard is selected
    expect(await dialogPage.isOptionSelected("discard")).toBe(true);

    // Click confirm (first time)
    await dialogPage.clickConfirm();

    // Verify confirmation message appears
    await expect(dialogPage.discardConfirmation).toBeVisible();
    await expect(dialogPage.confirmButton).toContainText("Confirm Discard");

    await expect(page).toHaveScreenshot("merge-workflow-dialog-discard-confirmation.png", {
      fullPage: true,
    });
  });

  test("should display view diff button when callback provided", async ({ page }) => {
    await openMergeWorkflowDialog(page, {
      showViewDiff: true,
    });

    // Verify button is visible
    expect(await dialogPage.hasViewDiffButton()).toBe(true);

    // Hover over button
    await dialogPage.viewDiffButton.hover();

    await expect(page).toHaveScreenshot("merge-workflow-dialog-view-diff-button.png", {
      fullPage: true,
    });
  });

  test("should display view commits button when callback provided", async ({ page }) => {
    await openMergeWorkflowDialog(page, {
      showViewCommits: true,
    });

    // Verify button is visible
    expect(await dialogPage.hasViewCommitsButton()).toBe(true);

    await expect(page).toHaveScreenshot("merge-workflow-dialog-view-commits-button.png", {
      fullPage: true,
    });
  });

  test("should display both action buttons", async ({ page }) => {
    await openMergeWorkflowDialog(page, {
      showViewDiff: true,
      showViewCommits: true,
    });

    // Verify both buttons are visible
    expect(await dialogPage.hasViewDiffButton()).toBe(true);
    expect(await dialogPage.hasViewCommitsButton()).toBe(true);

    await expect(page).toHaveScreenshot("merge-workflow-dialog-both-action-buttons.png", {
      fullPage: true,
    });
  });

  test("should display processing state", async ({ page }) => {
    await openMergeWorkflowDialog(page);

    // Set processing state
    await setMergeWorkflowProcessing(page, true);

    // Verify buttons are disabled
    expect(await dialogPage.isConfirmButtonDisabled()).toBe(true);
    expect(await dialogPage.isCancelButtonDisabled()).toBe(true);

    // Verify button text changes
    await expect(dialogPage.confirmButton).toContainText("Processing...");

    await expect(page).toHaveScreenshot("merge-workflow-dialog-processing.png", {
      fullPage: true,
    });
  });

  test("should display error message", async ({ page }) => {
    await openMergeWorkflowDialog(page);

    // Set error state
    await setMergeWorkflowError(page, "Failed to merge: conflict detected");

    // Verify error is visible
    expect(await dialogPage.hasError()).toBe(true);
    expect(await dialogPage.getErrorText()).toContain("Failed to merge");

    await expect(page).toHaveScreenshot("merge-workflow-dialog-error.png", {
      fullPage: true,
    });
  });

  test("should display singular commit count", async ({ page }) => {
    await openMergeWorkflowDialog(page, {
      completionData: {
        commitCount: 1,
        branchName: "ralphx/single-commit",
      },
    });

    // Verify singular form
    await expect(dialogPage.commitCount).toContainText("1 commit");
    await expect(dialogPage.commitCount).not.toContainText("commits");

    await expect(page).toHaveScreenshot("merge-workflow-dialog-single-commit.png", {
      fullPage: true,
    });
  });

  test("should display different project name", async ({ page }) => {
    await openMergeWorkflowDialog(page, {
      project: {
        id: "custom-project",
        name: "My Custom Project",
        path: "/path/to/custom",
        worktree_path: null,
        status: "active",
        created_at: "2026-01-31T10:00:00+00:00",
        updated_at: "2026-01-31T10:00:00+00:00",
      },
    });

    // Verify custom project name
    await expect(dialogPage.title).toContainText("My Custom Project");

    await expect(page).toHaveScreenshot("merge-workflow-dialog-custom-project.png", {
      fullPage: true,
    });
  });
});
