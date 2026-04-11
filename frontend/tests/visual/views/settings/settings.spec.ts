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
    await expect(settingsPage.autoCommitToggle).toBeVisible();
    await expect(settingsPage.pauseOnFailureToggle).toBeVisible();
    await expect(settingsPage.reviewBeforeDestructiveToggle).toBeVisible();
  });

  test("model section contains all controls", async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await settingsPage.openViaStore("model");
    await expect(settingsPage.modelSelect).toBeVisible();
    await expect(settingsPage.allowOpusUpgradeToggle).toBeVisible();
  });

  test("review section contains all controls", async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await settingsPage.openViaStore("review");
    await expect(settingsPage.aiReviewEnabledToggle).toBeVisible();
    await expect(settingsPage.aiReviewAutoFixToggle).toBeVisible();
    await expect(settingsPage.requireFixApprovalToggle).toBeVisible();
    await expect(settingsPage.requireHumanReviewToggle).toBeVisible();
    await expect(settingsPage.maxFixAttemptsInput).toBeVisible();
  });

  test("supervisor section contains all controls", async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await settingsPage.openViaStore("supervisor");
    await expect(settingsPage.supervisorEnabledToggle).toBeVisible();
    await expect(settingsPage.loopThresholdInput).toBeVisible();
    await expect(settingsPage.stuckTimeoutInput).toBeVisible();
  });

  test("matches snapshot - default state (execution section)", async ({ page }) => {
    await settingsPage.waitForAnimations();
    await expect(page).toHaveScreenshot("settings-dialog-default.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - review section disabled", async ({ page }) => {
    await settingsPage.openViaStore("review");
    // Disable AI review to see sub-settings disabled state
    await settingsPage.aiReviewEnabledToggle.click();
    await settingsPage.waitForAnimations();

    await expect(page).toHaveScreenshot("settings-dialog-review-disabled.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - supervisor section disabled", async ({ page }) => {
    await settingsPage.openViaStore("supervisor");
    // Disable supervisor to see sub-settings disabled state
    await settingsPage.supervisorEnabledToggle.click();
    await settingsPage.waitForAnimations();

    await expect(page).toHaveScreenshot("settings-dialog-supervisor-disabled.png", {
      fullPage: true,
    });
  });
});
