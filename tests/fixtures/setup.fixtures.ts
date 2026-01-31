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

export async function setupSettings(page: Page) {
  await setupApp(page);
  // Navigate to settings view
  await page.click('[data-testid="nav-settings"]');
  // Wait for settings view to load
  await page.waitForSelector('[data-testid="settings-view"]', { timeout: 10000 });
}

export async function setupExtensibility(page: Page) {
  await setupApp(page);
  // Navigate to extensibility view
  await page.click('[data-testid="nav-extensibility"]');
  // Wait for extensibility view to load
  await page.waitForSelector('[data-testid="extensibility-view"]', { timeout: 10000 });
}

export async function setupTaskDetail(page: Page) {
  await setupKanban(page);
  // Click the first task card to open detail overlay
  const firstTaskCard = page.locator('[data-testid^="task-card-"]').first();
  await firstTaskCard.click();
  // Wait for task detail overlay to load
  await page.waitForSelector('[data-testid="task-detail-overlay"]', { timeout: 10000 });
}

export async function setupReviewsPanel(page: Page) {
  await setupApp(page);
  // Click reviews toggle to open the panel
  await page.click('[data-testid="reviews-toggle"]');
  // Wait for reviews panel to load
  await page.waitForSelector('[data-testid="reviews-panel"]', { timeout: 10000 });
}
