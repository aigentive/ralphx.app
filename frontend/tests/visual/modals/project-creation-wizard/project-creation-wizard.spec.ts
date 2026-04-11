import { test, expect } from "@playwright/test";
import { ProjectCreationWizardPage } from "../../../pages/modals/project-creation-wizard.page";
import { setupApp } from "../../../fixtures/setup.fixtures";
import { openProjectCreationWizard } from "../../../helpers/project-creation-wizard.helpers";

/**
 * Visual regression tests for ProjectCreationWizard modal.
 *
 * The ProjectCreationWizard is now worktree-first.
 * Base branch and derived worktree path are visible by default; advanced
 * settings reveal the optional parent directory override.
 */

test.describe("ProjectCreationWizard", () => {
  let wizardPage: ProjectCreationWizardPage;

  test.beforeEach(async ({ page }) => {
    wizardPage = new ProjectCreationWizardPage(page);
    await setupApp(page);
  });

  test("renders modal with default worktree state", async ({ page }) => {
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

    // Worktree configuration is the default contract now
    await expect(wizardPage.baseBranchSelect).toBeVisible();
    await expect(wizardPage.worktreePathInput).toBeVisible();
    await expect(wizardPage.advancedSettingsTrigger).toBeVisible();
    await expect(wizardPage.worktreeParentInput).not.toBeVisible();

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

  test("reveals advanced worktree settings on demand", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    await expect(wizardPage.worktreeParentInput).not.toBeVisible();

    await wizardPage.openAdvancedSettings();

    await expect(wizardPage.worktreeParentInput).toBeVisible();
  });

  test("matches snapshot - default", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    // Wait for any animations
    await wizardPage.waitForAnimations();

    // Take snapshot
    await expect(wizardPage.modal).toHaveScreenshot("project-creation-wizard-default.png");
  });

  test("matches snapshot - advanced settings open", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    await wizardPage.openAdvancedSettings();
    await expect(wizardPage.worktreeParentInput).toBeVisible();

    // Wait for any animations
    await wizardPage.waitForAnimations();

    // Take snapshot
    await expect(wizardPage.modal).toHaveScreenshot("project-creation-wizard-advanced.png");
  });

  test("matches snapshot - with filled form", async ({ page }) => {
    await openProjectCreationWizard(page);
    await wizardPage.waitForModal();

    // Fill in the form
    await wizardPage.fillProjectName("My Test Project");
    await wizardPage.clickBrowseFolder();

    // Wait for directory to be filled
    await page.waitForTimeout(200);

    // Worktree settings are visible by default; advanced settings carry the custom parent path
    await wizardPage.selectBaseBranch("main");
    await wizardPage.openAdvancedSettings();
    await wizardPage.worktreeParentInput.fill("/Users/test/projects/.ralphx");

    // Wait for any animations
    await wizardPage.waitForAnimations();

    // Take snapshot
    await expect(wizardPage.modal).toHaveScreenshot("project-creation-wizard-filled.png");
  });
});
