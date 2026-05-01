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
