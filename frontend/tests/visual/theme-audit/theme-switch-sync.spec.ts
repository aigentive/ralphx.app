import { test, expect } from "@playwright/test";

async function settingsCardIconTileBackground(page: import("@playwright/test").Page) {
  return page
    .locator('[data-testid="settings-dialog"] div.p-2.rounded-lg.shrink-0')
    .first()
    .evaluate((node) => getComputedStyle(node).backgroundColor);
}

test("stored HC switches to Dark via the theme selector only", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("ralphx-theme", "high-contrast");
  });
  await page.goto("/");
  await page.waitForSelector('[data-testid="app-header"]', { timeout: 15000 });

  // Open Settings → Accessibility
  await page.evaluate(() => {
    const uiStore = (window as unknown as {
      __uiStore?: { getState(): { openModal(t: string): void } };
    }).__uiStore;
    uiStore?.getState().openModal("settings");
  });
  await page.waitForSelector('[data-testid="settings-dialog"]', { timeout: 10000 });
  await page.locator("text=Accessibility").first().click();
  await page.waitForTimeout(300);

  await expect(page.locator('[data-testid="theme-high-contrast"]')).toHaveCount(0);
  expect(await settingsCardIconTileBackground(page)).toBe("rgb(255, 255, 255)");

  // Pick Dark from dropdown.
  await page.locator('[data-testid="theme-selector"]').click();
  await page
    .locator('[role="option"]')
    .filter({ has: page.locator('span:text-is("Dark (default)")') })
    .click();
  await page.waitForTimeout(500);

  // And DOM + localStorage should say Dark.
  const state = await page.evaluate(() => ({
    attr: document.documentElement.getAttribute("data-theme"),
    ls: localStorage.getItem("ralphx-theme"),
  }));
  expect(state.attr).toBe("dark");
  expect(state.ls).toBe("dark");
});

test("Dark→HC→Dark roundtrip stays dropdown-only and ends on Dark", async ({ page }) => {
  await page.goto("/");
  await page.waitForSelector('[data-testid="app-header"]', { timeout: 15000 });

  await page.evaluate(() => {
    const uiStore = (window as unknown as {
      __uiStore?: { getState(): { openModal(t: string): void } };
    }).__uiStore;
    uiStore?.getState().openModal("settings");
  });
  await page.waitForSelector('[data-testid="settings-dialog"]', { timeout: 10000 });
  await page.locator("text=Accessibility").first().click();
  await page.waitForTimeout(300);

  await expect(page.locator('[data-testid="theme-high-contrast"]')).toHaveCount(0);

  // Dark → HC via dropdown
  await page.locator('[data-testid="theme-selector"]').click();
  await page
    .locator('[role="option"]')
    .filter({ has: page.locator('span:text-is("Dark (default)")') })
    .click();
  await page.waitForTimeout(300);

  await page.locator('[data-testid="theme-selector"]').click();
  await page
    .locator('[role="option"]')
    .filter({ has: page.locator('span:text-is("High contrast")') })
    .click();
  await page.waitForTimeout(300);
  expect(await settingsCardIconTileBackground(page)).toBe("rgb(255, 255, 255)");

  // HC → Dark via dropdown
  await page.locator('[data-testid="theme-selector"]').click();
  await page
    .locator('[role="option"]')
    .filter({ has: page.locator('span:text-is("Dark (default)")') })
    .click();
  await page.waitForTimeout(500);

  const state = await page.evaluate(() => ({
    attr: document.documentElement.getAttribute("data-theme"),
    ls: localStorage.getItem("ralphx-theme"),
  }));
  console.log("After roundtrip:", state);
  expect(state.attr).toBe("dark");
  expect(state.ls).toBe("dark");
  expect(await settingsCardIconTileBackground(page)).not.toBe("rgb(255, 255, 255)");
});
