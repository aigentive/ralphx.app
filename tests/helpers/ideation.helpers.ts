import { Page } from "@playwright/test";

/**
 * Helper to select the mock ideation session that has proposals.
 * The mock data in src/api-mock/ideation.ts creates a session with one proposal.
 * This helper waits for that data to load and waits for the proposal cards.
 */
export async function loadMockIdeationSession(page: Page) {
  // The mock creates a default session automatically
  // Wait for the session to load and display (ensureMockData in mock creates it)
  // Try waiting for session items to appear in the sidebar
  await page.waitForSelector('[data-testid^="session-item-"]', { timeout: 10000 });

  // Click the first session in the sidebar
  const firstSession = page.locator('[data-testid^="session-item-"]').first();
  await firstSession.click();

  // Wait for the session header to appear
  await page.waitForSelector('[data-testid="ideation-header"]', { timeout: 10000 });

  // Wait for proposal cards to appear
  await page.waitForSelector('[data-testid^="proposal-card-"]', { timeout: 10000 });
}
