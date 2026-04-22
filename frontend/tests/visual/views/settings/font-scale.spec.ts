/**
 * Font-scale computed-size spec.
 *
 * Verifies that switching Settings → Accessibility → Font size produces
 * monotonic root font-size growth (default < lg < xl) in both the Settings
 * surface and a non-Settings app-shell selector.
 *
 * Regression guard: this spec would fail against the pre-fix CSS where
 * `[data-font-scale="lg"] { font-size: 110%; }` resolves to ~17.6px
 * (110% of the browser 16px default), which is smaller than the app's
 * 18px baseline — inverting the expected order.
 *
 * Run: npx playwright test tests/visual/views/settings/font-scale.spec.ts
 */
import { test, expect, type Page } from "@playwright/test";
import { SettingsPage } from "../../../pages/settings.page";
import { setupApp } from "../../../fixtures/setup.fixtures";

async function getRootFontSize(page: Page): Promise<number> {
  return page.evaluate(() =>
    parseFloat(getComputedStyle(document.documentElement).fontSize),
  );
}

async function getElementFontSize(page: Page, selector: string): Promise<number> {
  return page.evaluate((sel) => {
    const el = document.querySelector(sel);
    if (!el) throw new Error(`Selector not found: ${sel}`);
    return parseFloat(getComputedStyle(el).fontSize);
  }, selector);
}

test.describe("Font scale — computed size assertions", () => {
  let settingsPage: SettingsPage;

  test.beforeEach(async ({ page }) => {
    settingsPage = new SettingsPage(page);
    await setupApp(page);
    // Reset to default so each test starts from the same baseline.
    await settingsPage.selectFontScale("default");
    // Close the settings dialog after reset.
    await settingsPage.closeModal();
  });

  test("default scale produces the 18px app baseline", async ({ page }) => {
    const rootPx = await getRootFontSize(page);
    expect(rootPx).toBeCloseTo(18, 0);
  });

  test("lg scale produces a root font size larger than default", async ({ page }) => {
    const defaultRootPx = await getRootFontSize(page);

    await settingsPage.selectFontScale("lg");
    await settingsPage.closeModal();

    const lgRootPx = await getRootFontSize(page);
    expect(lgRootPx).toBeCloseTo(19.8, 0);
    expect(lgRootPx).toBeGreaterThan(defaultRootPx);
  });

  test("xl scale produces a root font size larger than lg", async ({ page }) => {
    await settingsPage.selectFontScale("lg");
    await settingsPage.closeModal();
    const lgRootPx = await getRootFontSize(page);

    await settingsPage.selectFontScale("xl");
    await settingsPage.closeModal();
    const xlRootPx = await getRootFontSize(page);

    expect(xlRootPx).toBeCloseTo(22.5, 0);
    expect(xlRootPx).toBeGreaterThan(lgRootPx);
  });

  test("default < lg < xl root font size — full monotonic assertion", async ({ page }) => {
    const defaultRootPx = await getRootFontSize(page);

    await settingsPage.selectFontScale("lg");
    await settingsPage.closeModal();
    const lgRootPx = await getRootFontSize(page);

    await settingsPage.selectFontScale("xl");
    await settingsPage.closeModal();
    const xlRootPx = await getRootFontSize(page);

    expect(lgRootPx).toBeGreaterThan(defaultRootPx);
    expect(xlRootPx).toBeGreaterThan(lgRootPx);
  });

  test("Settings label text grows monotonically across scales", async ({ page }) => {
    // The Settings section card title uses rem-based text utilities that
    // scale with the root font-size.  We read the computed font size of the
    // first visible text node inside the settings dialog.
    const getSettingsLabelSize = async () => {
      await settingsPage.openViaStore("accessibility");
      const size = await getElementFontSize(
        page,
        '[data-testid="settings-dialog"] h2',
      );
      await settingsPage.closeModal();
      return size;
    };

    const defaultSize = await getSettingsLabelSize();

    await settingsPage.selectFontScale("lg");
    await settingsPage.closeModal();
    const lgSize = await getSettingsLabelSize();

    await settingsPage.selectFontScale("xl");
    await settingsPage.closeModal();
    const xlSize = await getSettingsLabelSize();

    expect(lgSize).toBeGreaterThan(defaultSize);
    expect(xlSize).toBeGreaterThan(lgSize);
  });

  test("app header title (non-Settings surface) grows monotonically across scales", async ({
    page,
  }) => {
    // The app header branding <h1> uses text-xl (1.25rem), so it scales with
    // the root font-size.  This is the non-Settings verification surface.
    const getHeaderSize = () => getElementFontSize(page, '[data-testid="app-header"] h1');

    const defaultSize = await getHeaderSize();

    await settingsPage.selectFontScale("lg");
    await settingsPage.closeModal();
    const lgSize = await getHeaderSize();

    await settingsPage.selectFontScale("xl");
    await settingsPage.closeModal();
    const xlSize = await getHeaderSize();

    expect(lgSize).toBeGreaterThan(defaultSize);
    expect(xlSize).toBeGreaterThan(lgSize);
  });
});
