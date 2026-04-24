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
    await expect(page.getByTestId("design-styleguide-group-colors")).toContainText(
      "Primary palette",
    );
    await saveShot(page, "design-desktop-initial");

    await page.getByTestId("design-new-system").click();
    await expect(page.getByTestId("integrated-chat-header")).toContainText("draft / 0 sources");
    await page.getByTestId("design-generate-styleguide").click();
    await expect(page.getByTestId("integrated-chat-header")).toContainText("ready / 0 sources");
    await saveShot(page, "design-desktop-generated");

    const buttonRow = page.getByTestId("design-styleguide-row-components.buttons");
    await buttonRow.click();
    await expect(page.getByTestId("design-component-preview")).toBeVisible();

    await page.getByTestId("design-needs-work-components.buttons").click();
    await page.getByPlaceholder("Feedback").fill("Use the app's compact 8px control radius.");
    await page.getByText("Send feedback to Design").click();
    await expect(buttonRow).toContainText("needs work");

    const paletteRow = page.getByTestId("design-styleguide-row-colors.primary_palette");
    await paletteRow.click();
    await page.getByTestId("design-approve-colors.primary_palette").click();
    await expect(paletteRow).toContainText("approved");
    await saveShot(page, "design-desktop-reviewed");
  });

  test("captures the Design workspace in a compact viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openDesign(page);

    await expect(page.getByTestId("design-sidebar")).toBeVisible();
    await expect(page.getByTestId("design-styleguide-pane")).toBeVisible();
    await saveShot(page, "design-compact-initial");
  });
});
