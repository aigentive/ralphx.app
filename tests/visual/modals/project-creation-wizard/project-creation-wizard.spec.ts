import { test, expect } from "@playwright/test";
import { ProjectCreationWizardPage } from "../../../pages/modals/project-creation-wizard.page";
import { setupApp } from "../../../fixtures/setup.fixtures";
import { openProjectCreationWizard } from "../../../helpers/project-creation-wizard.helpers";

/**
 * Visual regression tests for ProjectCreationWizard modal.
 *
 * The ProjectCreationWizard allows users to create new projects with two Git modes:
 * - Local: Work directly in the current branch
 * - Worktree: Create an isolated worktree for RalphX
 */

test.describe("ProjectCreationWizard", () => {
  let wizardPage: ProjectCreationWizardPage;

  test.beforeEach(async ({ page }) => {
    wizardPage = new ProjectCreationWizardPage(page);
    await setupApp(page);
  });

  test("renders modal with default state (local mode)", async ({ page }) => {
    await openProjectCreationWizard(page);

    // Wait for modal to appear
    await wizardPage.waitForModal();

    // Verify modal is visible
    await expect(wizardPage.modal).toBeVisible();

    // Verify title
    await expect(wizardPage.title).toBeVisible();

    // Verify form fields are present
    await expect(wizardPage.projectNameInput).toBeVisible();
    await expect(wizardPage.workingDirectoryInput).toBeVisible();
    await expect(wizardPage.browseFolderButton).toBeVisible();

    // Verify git mode radios
    await expect(wizardPage.gitModeLocalRadio).toBeVisible();
    await expect(wizardPage.gitModeWorktreeRadio).toBeVisible();

    // Local mode is selected by default - worktree fields should not be visible
    const worktreeFieldsVisible = await wizardPage.areWorktreeFieldsVisible();
    expect(worktreeFieldsVisible).toBe(false);

    // Verify buttons
    await expect(wizardPage.createButton).toBeVisible();
    await expect(wizardPage.cancelButton).toBeVisible();
  });

  test("fills project name and selects folder", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    // Fill project name
    await wizardPage.fillProjectName("My Test Project");
    await expect(wizardPage.projectNameInput).toHaveValue("My Test Project");

    // Click browse folder button (mock will auto-fill)
    await wizardPage.clickBrowseFolder();

    // Wait for directory to be filled
    await page.waitForTimeout(200);

    // Verify working directory is filled
    const workingDir = await wizardPage.getWorkingDirectory();
    expect(workingDir).toContain("/Users/test/projects/test-project");
  });

  test("switches to worktree mode and shows additional fields", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    // Initially in local mode
    let worktreeFieldsVisible = await wizardPage.areWorktreeFieldsVisible();
    expect(worktreeFieldsVisible).toBe(false);

    // Switch to worktree mode
    await wizardPage.selectWorktreeMode();

    // Wait for fields to appear
    await wizardPage.baseBranchSelect.waitFor({ state: "visible" });

    // Verify worktree fields are now visible
    await expect(wizardPage.baseBranchSelect).toBeVisible();
    await expect(wizardPage.worktreeBranchInput).toBeVisible();
    await expect(wizardPage.worktreePathInput).toBeVisible();
  });

  test("matches snapshot - local mode default", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    // Wait for any animations
    await wizardPage.waitForAnimations();

    // Take snapshot
    await expect(wizardPage.modal).toHaveScreenshot("project-creation-wizard-local-mode.png");
  });

  test("matches snapshot - worktree mode", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    // Switch to worktree mode
    await wizardPage.selectWorktreeMode();

    // Wait for worktree fields to appear
    await wizardPage.baseBranchSelect.waitFor({ state: "visible" });

    // Wait for any animations
    await wizardPage.waitForAnimations();

    // Take snapshot
    await expect(wizardPage.modal).toHaveScreenshot("project-creation-wizard-worktree-mode.png");
  });

  test("matches snapshot - with filled form", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    // Fill in the form
    await wizardPage.fillProjectName("My Test Project");
    await wizardPage.clickBrowseFolder();

    // Wait for directory to be filled
    await page.waitForTimeout(200);

    // Switch to worktree mode
    await wizardPage.selectWorktreeMode();
    await wizardPage.baseBranchSelect.waitFor({ state: "visible" });

    // Fill worktree fields
    await wizardPage.selectBaseBranch("main");
    await wizardPage.fillWorktreeBranch("ralphx/my-feature");

    // Wait for any animations
    await wizardPage.waitForAnimations();

    // Take snapshot
    await expect(wizardPage.modal).toHaveScreenshot("project-creation-wizard-filled.png");
  });
});
