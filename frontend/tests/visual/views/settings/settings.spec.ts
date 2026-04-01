import { test, expect } from "@playwright/test";
import { SettingsPage } from "../../../pages/settings.page";
import { setupSettings } from "../../../fixtures/setup.fixtures";

test.describe("Settings View", () => {
  let settingsPage: SettingsPage;

  test.beforeEach(async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await setupSettings(page);
  });

  test("renders settings view layout", async () => {
    await expect(settingsPage.settingsView).toBeVisible();
    await expect(settingsPage.settingsTitle).toBeVisible();
  });

  test("displays all four settings sections", async () => {
    await expect(settingsPage.executionSection).toBeVisible();
    await expect(settingsPage.modelSection).toBeVisible();
    await expect(settingsPage.reviewSection).toBeVisible();
    await expect(settingsPage.supervisorSection).toBeVisible();
  });

  test("execution section contains all controls", async () => {
    await expect(settingsPage.maxConcurrentTasksInput).toBeVisible();
    await expect(settingsPage.autoCommitToggle).toBeVisible();
    await expect(settingsPage.pauseOnFailureToggle).toBeVisible();
    await expect(settingsPage.reviewBeforeDestructiveToggle).toBeVisible();
  });

  test("model section contains all controls", async () => {
    await expect(settingsPage.modelSelect).toBeVisible();
    await expect(settingsPage.allowOpusUpgradeToggle).toBeVisible();
  });

  test("review section contains all controls", async () => {
    await expect(settingsPage.aiReviewEnabledToggle).toBeVisible();
    await expect(settingsPage.aiReviewAutoFixToggle).toBeVisible();
    await expect(settingsPage.requireFixApprovalToggle).toBeVisible();
    await expect(settingsPage.requireHumanReviewToggle).toBeVisible();
    await expect(settingsPage.maxFixAttemptsInput).toBeVisible();
  });

  test("supervisor section contains all controls", async () => {
    await expect(settingsPage.supervisorEnabledToggle).toBeVisible();
    await expect(settingsPage.loopThresholdInput).toBeVisible();
    await expect(settingsPage.stuckTimeoutInput).toBeVisible();
  });

  test("matches snapshot - default state", async ({ page }) => {
    await settingsPage.waitForAnimations();
    await expect(page).toHaveScreenshot("settings-view-default.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - review section disabled", async ({ page }) => {
    // Disable AI review to see sub-settings disabled state
    await settingsPage.aiReviewEnabledToggle.click();
    await settingsPage.waitForAnimations();

    await expect(page).toHaveScreenshot("settings-view-review-disabled.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - supervisor section disabled", async ({ page }) => {
    // Disable supervisor to see sub-settings disabled state
    await settingsPage.supervisorEnabledToggle.click();
    await settingsPage.waitForAnimations();

    await expect(page).toHaveScreenshot("settings-view-supervisor-disabled.png", {
      fullPage: true,
    });
  });
});
