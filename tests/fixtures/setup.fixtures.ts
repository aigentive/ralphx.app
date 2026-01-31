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

export async function setupIdeation(page: Page) {
  await setupApp(page);
  // Navigate to ideation view
  await page.click('[data-testid="nav-ideation"]');
  // Wait for ideation view to load
  await page.waitForSelector('[data-testid="ideation-view"]', { timeout: 10000 });
}

export async function setupActivity(page: Page) {
  await setupApp(page);
  // Navigate to activity view
  await page.click('[data-testid="nav-activity"]');
  // Wait for activity view to load
  await page.waitForSelector('[data-testid="activity-view"]', { timeout: 10000 });
}
