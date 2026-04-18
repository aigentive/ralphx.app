import { test, expect } from "@playwright/test";

test("HC switch unchecks when Dark is picked from dropdown while in HC", async ({ page }) => {
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

  // BEFORE: in HC — HC switch should be checked.
  const beforeSwitchChecked = await page
    .locator('[data-testid="theme-high-contrast"]')
    .getAttribute("data-state");
  console.log("HC switch before dropdown pick:", beforeSwitchChecked);
  expect(beforeSwitchChecked).toBe("checked");

  // Pick Dark from dropdown.
  await page.locator('[data-testid="theme-selector"]').click();
  await page
    .locator('[role="option"]')
    .filter({ has: page.locator('span:text-is("Dark (default)")') })
    .click();
  await page.waitForTimeout(500);

  // AFTER: HC switch should be UNCHECKED.
  const afterSwitchChecked = await page
    .locator('[data-testid="theme-high-contrast"]')
    .getAttribute("data-state");
  console.log("HC switch after Dark pick:", afterSwitchChecked);
  expect(afterSwitchChecked).toBe("unchecked");

  // And DOM + localStorage should say Dark.
  const state = await page.evaluate(() => ({
    attr: document.documentElement.getAttribute("data-theme"),
    ls: localStorage.getItem("ralphx-theme"),
  }));
  expect(state.attr).toBe("dark");
  expect(state.ls).toBe("dark");
});

test("Dark→HC→Dark roundtrip leaves HC switch unchecked and theme Dark", async ({ page }) => {
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

  // HC → Dark via dropdown
  await page.locator('[data-testid="theme-selector"]').click();
  await page
    .locator('[role="option"]')
    .filter({ has: page.locator('span:text-is("Dark (default)")') })
    .click();
  await page.waitForTimeout(500);

  const afterSwitch = await page
    .locator('[data-testid="theme-high-contrast"]')
    .getAttribute("data-state");
  const state = await page.evaluate(() => ({
    attr: document.documentElement.getAttribute("data-theme"),
    ls: localStorage.getItem("ralphx-theme"),
  }));
  console.log("After roundtrip:", { afterSwitch, ...state });
  expect(afterSwitch).toBe("unchecked");
  expect(state.attr).toBe("dark");
});
