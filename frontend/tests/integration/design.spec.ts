import { expect, test, type Page } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import { join } from "node:path";

const OUTPUT_ROOT = join(
  process.cwd(),
  "..",
  ".artifacts",
  "specs",
  "ralphx-design-feature",
  "playwright",
);

async function openDesign(page: Page) {
  await page.goto("/");
  await page.waitForSelector('[data-testid="app-header"]', { timeout: 15000 });
  await page.getByTestId("nav-design").click();
  await expect(page.getByTestId("design-view")).toBeVisible();
}

async function saveShot(page: Page, name: string) {
  await mkdir(OUTPUT_ROOT, { recursive: true });
  await page.screenshot({
    path: join(OUTPUT_ROOT, `${name}.png`),
    fullPage: true,
  });
}

test.describe("Design workspace", () => {
  test("validates the create, generate, review, and feedback flow", async ({ page }) => {
    await openDesign(page);

    await expect(page.getByTestId("design-sidebar")).toBeVisible();
    await expect(page.getByTestId("design-styleguide-pane")).toBeVisible();
    await expect(page.getByTestId("integrated-chat-panel")).toBeVisible();
    await expect(page.getByTestId("design-styleguide-empty")).toContainText("No styleguide rows yet");
    await saveShot(page, "design-desktop-initial");

    await page.getByTestId("design-new-system").click();
    await expect(page.getByTestId("design-source-composer")).toBeVisible();
    await saveShot(page, "design-source-composer");
    await page.getByTestId("design-primary-paths").fill("frontend/src");
    await page.getByTestId("design-create-from-sources").click();
    await expect(page.getByTestId("integrated-chat-header")).toContainText("draft · 1 source");
    await page.getByTestId("design-generate-styleguide").click();
    await expect(page.getByTestId("integrated-chat-header")).toContainText("ready · 1 source");
    await expect(page.getByTestId("integrated-chat-header")).toContainText("Design steward");
    await page.getByTestId("design-export-package").click();
    await expect(page.getByTestId("design-export-result")).toContainText("Export ready");
    await expect(page.getByTestId("design-download-export-package")).toContainText("Download JSON");
    await expect(page.getByTestId("design-composer-surface")).toHaveAttribute(
      "data-design-system-id",
      "design-system-project-mock-1-2",
    );

    const chatInput = page.getByTestId("chat-input-textarea");
    await expect(chatInput).toBeVisible();
    await expect(chatInput).toHaveAttribute(
      "placeholder",
      /Ask Design to analyze, refine, or generate a screen/,
    );
    await chatInput.fill("Review the primary palette caveat before I approve it.");
    await expect(page.getByTestId("chat-input-send")).toBeEnabled();
    await page.getByTestId("chat-input-send").click();
    await expect(page.getByTestId("integrated-chat-panel")).toContainText(
      "Review the primary palette caveat before I approve it.",
    );

    const styleguidePane = page.getByTestId("design-styleguide-resizable-pane");
    const resizeHandle = page.getByTestId("design-styleguide-resize-handle");
    const paneBeforeResize = await styleguidePane.boundingBox();
    const resizeHandleBox = await resizeHandle.boundingBox();
    expect(paneBeforeResize).not.toBeNull();
    expect(resizeHandleBox).not.toBeNull();
    await page.mouse.move(
      resizeHandleBox!.x + resizeHandleBox!.width / 2,
      resizeHandleBox!.y + resizeHandleBox!.height / 2,
    );
    await page.mouse.down();
    await page.mouse.move(
      resizeHandleBox!.x + resizeHandleBox!.width / 2 + 120,
      resizeHandleBox!.y + resizeHandleBox!.height / 2,
    );
    await page.mouse.up();
    const paneAfterResize = await styleguidePane.boundingBox();
    expect(paneAfterResize).not.toBeNull();
    expect(paneAfterResize!.width).toBeLessThan(paneBeforeResize!.width - 40);

    await saveShot(page, "design-desktop-generated");

    const buttonRow = page.getByTestId("design-styleguide-row-components.buttons");
    await buttonRow.click();
    await expect(page.getByTestId("design-component-preview")).toBeVisible();
    await expect(page.getByTestId("design-component-preview")).toContainText("Button");
    await expect(page.getByTestId("design-component-preview")).toContainText("default");

    const typeRow = page.getByTestId("design-styleguide-row-type.typography_scale");
    await typeRow.click();
    await expect(page.getByTestId("design-typography-preview")).toBeVisible();
    await expect(page.getByTestId("design-preview-kind")).toContainText(
      "typography sample / 3 sources",
    );

    await buttonRow.click();
    await page.getByTestId("design-open-full-preview-components.buttons").click();
    await expect(page.getByTestId("design-focused-item-drawer")).toContainText("Buttons");
    await page.getByTestId("design-close-focused-preview").click();
    await page.getByTestId("design-generate-artifact-components.buttons").click();
    await expect(page.getByTestId("design-generated-artifact-result")).toContainText(
      "Generated component artifact",
    );

    await page.getByTestId("design-needs-work-components.buttons").click();
    await page.getByPlaceholder("Feedback").fill("Use the app's compact 8px control radius.");
    await page.getByText("Send feedback to Design").click();
    await expect(buttonRow).toContainText("needs work");

    const paletteRow = page.getByTestId("design-styleguide-row-colors.primary_palette");
    await paletteRow.click();
    await page.getByTestId("design-approve-colors.primary_palette").click();
    await expect(paletteRow).toContainText("approved");
    await saveShot(page, "design-desktop-reviewed");

    await page.getByTestId("design-import-package").click();
    await expect(page.getByTestId("design-package-import-dialog")).toBeVisible();
    await page.getByTestId("design-import-package-artifact-id").fill("export-design-system-project-mock-1-2");
    await page.getByTestId("design-import-name").fill("Imported Demo UI");
    await page.getByTestId("design-import-package-submit").click();
    await expect(page.getByTestId("integrated-chat-header")).toContainText("Imported Demo UI");
    await expect(page.getByTestId("integrated-chat-header")).toContainText("ready · 1 source");
    await saveShot(page, "design-desktop-imported");
  });

  test("captures the Design workspace in a compact viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openDesign(page);

    await expect(page.getByTestId("design-sidebar")).toBeVisible();
    await expect(page.getByTestId("design-styleguide-pane")).toBeVisible();
    await saveShot(page, "design-compact-initial");
  });
});
