import { test, expect } from "@playwright/test";
import { ExtensibilityPage } from "../../../pages/extensibility.page";
import { setupExtensibility } from "../../../fixtures/setup.fixtures";

test.describe("Extensibility View", () => {
  let extensibilityPage: ExtensibilityPage;

  test.beforeEach(async ({ page }) => {
    extensibilityPage = new ExtensibilityPage(page);
    await setupExtensibility(page);
  });

  test("renders extensibility view layout", async () => {
    await expect(extensibilityPage.extensibilityView).toBeVisible();
    await expect(extensibilityPage.tabNavigation).toBeVisible();
  });

  test("displays all four tabs", async () => {
    await expect(extensibilityPage.workflowsTab).toBeVisible();
    await expect(extensibilityPage.artifactsTab).toBeVisible();
    await expect(extensibilityPage.researchTab).toBeVisible();
    await expect(extensibilityPage.methodologiesTab).toBeVisible();
  });

  test("workflows tab is active by default", async () => {
    const isActive = await extensibilityPage.isTabActive("workflows");
    expect(isActive).toBe(true);
    await expect(extensibilityPage.workflowsPanel).toBeVisible();
  });

  test("can switch to artifacts tab", async () => {
    await extensibilityPage.switchTab("artifacts");
    const isActive = await extensibilityPage.isTabActive("artifacts");
    expect(isActive).toBe(true);
    await expect(extensibilityPage.artifactsPanel).toBeVisible();
  });

  test("can switch to research tab", async () => {
    await extensibilityPage.switchTab("research");
    const isActive = await extensibilityPage.isTabActive("research");
    expect(isActive).toBe(true);
    await expect(extensibilityPage.researchPanel).toBeVisible();
  });

  test("can switch to methodologies tab", async () => {
    await extensibilityPage.switchTab("methodologies");
    const isActive = await extensibilityPage.isTabActive("methodologies");
    expect(isActive).toBe(true);
    await expect(extensibilityPage.methodologiesPanel).toBeVisible();
  });

  test("matches snapshot - workflows tab", async ({ page }) => {
    await extensibilityPage.waitForAnimations();
    await expect(page).toHaveScreenshot("extensibility-workflows.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - artifacts tab", async ({ page }) => {
    await extensibilityPage.switchTab("artifacts");
    await extensibilityPage.waitForAnimations();
    await expect(page).toHaveScreenshot("extensibility-artifacts.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - research tab", async ({ page }) => {
    await extensibilityPage.switchTab("research");
    await extensibilityPage.waitForAnimations();
    await expect(page).toHaveScreenshot("extensibility-research.png", {
      fullPage: true,
    });
  });

  test("matches snapshot - methodologies tab", async ({ page }) => {
    await extensibilityPage.switchTab("methodologies");
    await extensibilityPage.waitForAnimations();
    await expect(page).toHaveScreenshot("extensibility-methodologies.png", {
      fullPage: true,
    });
  });
});
