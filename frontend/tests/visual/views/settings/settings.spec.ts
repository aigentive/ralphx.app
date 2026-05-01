import { test, expect } from "@playwright/test";
import { SettingsPage } from "../../../pages/settings.page";
import { setupSettings } from "../../../fixtures/setup.fixtures";

test.describe("Settings Dialog", () => {
  let settingsPage: SettingsPage;

  test.beforeEach(async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await setupSettings(page);
  });

  test("renders settings dialog layout", async () => {
    await expect(settingsPage.settingsDialog).toBeVisible();
    await expect(settingsPage.settingsTitle).toBeVisible();
  });

  test("renders above the underlying view (modal overlay)", async ({ page }) => {
    // The kanban/current view is still mounted behind the modal
    await expect(settingsPage.settingsDialog).toBeVisible();
    // Dialog should have highest z-index (rendered in portal above app content)
    const zIndex = await page.evaluate(() => {
      const dialog = document.querySelector('[data-testid="settings-dialog"]');
      if (!dialog) return null;
      return getComputedStyle(dialog.closest('[role="dialog"]') ?? dialog).zIndex;
    });
    expect(zIndex).not.toBeNull();
  });

  test("execution section contains all controls", async () => {
    await expect(settingsPage.maxConcurrentTasksInput).toBeVisible();
    await expect(settingsPage.projectIdeationMaxInput).toBeVisible();
  });

  test("global capacity section contains all controls", async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await settingsPage.openViaStore("global-execution");
    await expect(settingsPage.globalMaxConcurrentInput).toBeVisible();
    await expect(settingsPage.globalIdeationMaxInput).toBeVisible();
    await expect(settingsPage.allowIdeationBorrowIdleExecutionToggle).toBeVisible();
  });

  test("review section contains all controls", async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await settingsPage.openViaStore("review");
    await expect(settingsPage.requireHumanReviewToggle).toBeVisible();
    await expect(settingsPage.maxFixAttemptsInput).toBeVisible();
    await expect(settingsPage.maxRevisionCyclesInput).toBeVisible();
  });

  test("external MCP section contains all controls", async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await settingsPage.openViaStore("external-mcp");
    await expect(settingsPage.externalMcpEnabledToggle).toBeVisible();
    await expect(settingsPage.externalMcpHostInput).toBeVisible();
    await expect(settingsPage.externalMcpPortInput).toBeVisible();
    await expect(settingsPage.externalMcpAuthTokenInput).toBeVisible();
    await expect(settingsPage.externalMcpNodePathInput).toBeVisible();
    await expect(settingsPage.externalMcpSaveButton).toBeVisible();
  });

  test("repository section shows git auth repair actions", async ({ page }) => {
    await page.evaluate(() => {
      window.__mockGhAuthStatus = true;
      window.__mockGitAuthDiagnostics = {
        fetchUrl: "https://github.com/mock/project.git",
        pushUrl: "git@github.com:mock/project.git",
        fetchKind: "HTTPS",
        pushKind: "SSH",
        mixedAuthModes: true,
        canSwitchToSsh: true,
        suggestedSshUrl: "git@github.com:mock/project.git",
      };
    });
    await settingsPage.openViaStore("repository");

    const repairPanel = page.getByTestId("git-auth-repair-panel");
    await repairPanel.scrollIntoViewIfNeeded();
    await expect(repairPanel).toBeVisible();
    await expect(page.getByTestId("git-auth-switch-ssh")).toBeVisible();
    await expect(page.getByTestId("git-auth-setup-gh")).toBeVisible();

    await settingsPage.waitForAnimations();
    await expect(repairPanel).toHaveScreenshot("settings-repository-git-auth-repair-panel.png", {
      maxDiffPixelRatio: 0.01,
    });
  });

  test("matches snapshot - default state (execution section)", async ({ page }) => {
    await settingsPage.waitForAnimations();
    await expect(page).toHaveScreenshot("settings-dialog-default.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - review section disabled", async ({ page }) => {
    await settingsPage.openViaStore("review");
    await settingsPage.requireHumanReviewToggle.click();
    await settingsPage.waitForAnimations();

    await expect(page).toHaveScreenshot("settings-dialog-review-disabled.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - external MCP section disabled", async ({ page }) => {
    await settingsPage.openViaStore("external-mcp");
    await settingsPage.externalMcpEnabledToggle.click();
    await settingsPage.waitForAnimations();

    await expect(page).toHaveScreenshot("settings-dialog-external-mcp-disabled.png", {
      fullPage: true,
    });
  });
});
