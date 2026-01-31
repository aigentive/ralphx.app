import { Page } from "@playwright/test";

export async function waitForNetworkIdle(page: Page, timeout = 5000) {
  await page.waitForLoadState("networkidle", { timeout });
}

export async function waitForAnimationsComplete(page: Page) {
  await page.waitForTimeout(500);
  // Could add check for CSS animations complete
}
