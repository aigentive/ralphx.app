import { test, expect } from "@playwright/test";
import { PermissionDialogPage } from "../../../pages/modals/permission-dialog.page";
import {
  triggerPermissionDialog,
  createBashPermissionRequest,
  createWritePermissionRequest,
  createEditPermissionRequest,
  createReadPermissionRequest,
  createLongContentPermissionRequest,
} from "../../../helpers/permission.helpers";

test.describe("PermissionDialog", () => {
  let permissionDialog: PermissionDialogPage;

  test.beforeEach(async ({ page }) => {
    permissionDialog = new PermissionDialogPage(page);
    await page.goto("http://localhost:5173");
    await permissionDialog.waitForApp();
  });

  test("renders Bash permission request correctly", async ({ page }) => {
    const request = createBashPermissionRequest();
    await triggerPermissionDialog(page, request);

    await permissionDialog.waitForDialog();
    await expect(permissionDialog.dialog).toBeVisible();
    await expect(permissionDialog.title).toBeVisible();

    // Verify tool name is displayed
    const toolName = await permissionDialog.getToolName();
    expect(toolName).toContain("Bash");

    // Verify context is displayed
    const context = await permissionDialog.getContext();
    expect(context).toBe("Agent needs to commit code changes");

    // Verify input preview shows the command
    const inputPreview = await permissionDialog.getInputPreview();
    expect(inputPreview).toContain('git commit -m "feat: add new feature"');

    // Wait for animations to complete
    await permissionDialog.waitForAnimations();

    // Visual regression snapshot
    await expect(page).toHaveScreenshot("permission-dialog-bash.png");
  });

  test("renders Write permission request correctly", async ({ page }) => {
    const request = createWritePermissionRequest();
    await triggerPermissionDialog(page, request);

    await permissionDialog.waitForDialog();

    // Verify tool name
    const toolName = await permissionDialog.getToolName();
    expect(toolName).toContain("Write");

    // Verify file path is shown
    const inputPreview = await permissionDialog.getInputPreview();
    expect(inputPreview).toContain("/path/to/new-file.ts");
    expect(inputPreview).toContain("export function example()");

    // Wait for animations
    await permissionDialog.waitForAnimations();

    // Visual regression snapshot
    await expect(page).toHaveScreenshot("permission-dialog-write.png");
  });

  test("renders Edit permission request correctly", async ({ page }) => {
    const request = createEditPermissionRequest();
    await triggerPermissionDialog(page, request);

    await permissionDialog.waitForDialog();

    // Verify tool name
    const toolName = await permissionDialog.getToolName();
    expect(toolName).toContain("Edit");

    // Verify file path and old/new strings are shown
    const inputPreview = await permissionDialog.getInputPreview();
    expect(inputPreview).toContain("/path/to/existing-file.ts");
    expect(inputPreview).toContain("const value = 'old'");
    expect(inputPreview).toContain("const value = 'new'");

    // Wait for animations
    await permissionDialog.waitForAnimations();

    // Visual regression snapshot
    await expect(page).toHaveScreenshot("permission-dialog-edit.png");
  });

  test("renders Read permission request correctly", async ({ page }) => {
    const request = createReadPermissionRequest();
    await triggerPermissionDialog(page, request);

    await permissionDialog.waitForDialog();

    // Verify tool name
    const toolName = await permissionDialog.getToolName();
    expect(toolName).toContain("Read");

    // Verify file path is shown
    const inputPreview = await permissionDialog.getInputPreview();
    expect(inputPreview).toContain("/path/to/sensitive-file.env");

    // Wait for animations
    await permissionDialog.waitForAnimations();

    // Visual regression snapshot
    await expect(page).toHaveScreenshot("permission-dialog-read.png");
  });

  test("truncates long content correctly", async ({ page }) => {
    const request = createLongContentPermissionRequest();
    await triggerPermissionDialog(page, request);

    await permissionDialog.waitForDialog();

    // Verify content is truncated at 200 characters
    const inputPreview = await permissionDialog.getInputPreview();
    expect(inputPreview).toContain("...");
    expect(inputPreview.length).toBeLessThan(300); // Should be truncated + ellipsis

    // Wait for animations
    await permissionDialog.waitForAnimations();

    // Visual regression snapshot
    await expect(page).toHaveScreenshot("permission-dialog-truncated.png");
  });

  test("shows queue count when multiple requests pending", async ({ page }) => {
    // Queue multiple requests
    const request1 = createBashPermissionRequest();
    const request2 = createWritePermissionRequest();
    const request3 = createEditPermissionRequest();

    await triggerPermissionDialog(page, request1);
    await triggerPermissionDialog(page, request2);
    await triggerPermissionDialog(page, request3);

    await permissionDialog.waitForDialog();

    // Should show queue count badge
    const queueCount = await permissionDialog.getQueueCount();
    expect(queueCount).toBeTruthy();
    expect(queueCount).toContain("2"); // 2 more requests pending

    // Wait for animations
    await permissionDialog.waitForAnimations();

    // Visual regression snapshot
    await expect(page).toHaveScreenshot("permission-dialog-queue.png");
  });

  test("handles Allow button click", async ({ page }) => {
    const request = createBashPermissionRequest();
    await triggerPermissionDialog(page, request);

    await permissionDialog.waitForDialog();
    await permissionDialog.clickAllow();

    // Dialog should close after Allow
    await permissionDialog.waitForDialogToClose();
    await expect(permissionDialog.dialog).not.toBeVisible();
  });

  test("handles Deny button click", async ({ page }) => {
    const request = createBashPermissionRequest();
    await triggerPermissionDialog(page, request);

    await permissionDialog.waitForDialog();
    await permissionDialog.clickDeny();

    // Dialog should close after Deny
    await permissionDialog.waitForDialogToClose();
    await expect(permissionDialog.dialog).not.toBeVisible();
  });
});
