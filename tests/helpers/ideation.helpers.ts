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

/**
 * Helper to open the ProposalEditModal for the first proposal in the loaded session.
 * Must be called after loadMockIdeationSession.
 */
export async function openProposalEditModal(page: Page) {
  const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();

  // Hover to reveal edit button
  await firstProposalCard.hover();

  // Wait a moment for hover state to activate
  await page.waitForTimeout(200);

  // Click the edit button (second button in the card - first is checkbox, second is edit icon)
  const editButton = firstProposalCard.locator('button').nth(1);
  await editButton.click();

  // Wait for modal to appear
  await page.waitForSelector('[data-testid="proposal-edit-modal"]', { timeout: 5000 });
}
