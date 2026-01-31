import { Page } from "@playwright/test";

export async function setupApp(page: Page) {
  await page.goto("/");
  await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });
}

export async function setupKanban(page: Page) {
  await setupApp(page);
  // Wait for kanban-specific elements
  await page.waitForSelector('[data-testid^="task-card-"]');
}
