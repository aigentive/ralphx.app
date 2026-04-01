/**
 * Test helpers for ReviewDetailModal
 *
 * Uses React state manipulation via exposed test helper
 * (similar to task-detail.helpers.ts approach - see Phase 52 for pattern)
 */

import type { Page } from "@playwright/test";
import { setupKanban } from "../fixtures/setup.fixtures";

/**
 * Opens ReviewDetailModal programmatically by:
 * 1. Setting up kanban view
 * 2. Finding a task with review_passed status
 * 3. Using window.__openReviewDetailModal test helper
 */
export async function openReviewDetailModal(page: Page): Promise<void> {
  // Setup kanban view
  await setupKanban(page);

  // Find a task with review_passed status
  const taskId = await page.evaluate(() => {
    const store = (window as any).__mockStore;
    if (!store) {
      throw new Error("Mock store not available");
    }

    const reviewStatuses = ["review_passed", "escalated"];
    const tasks = Array.from(store.tasks.values());
    const reviewTask = tasks.find((t: any) =>
      reviewStatuses.includes(t.internalStatus)
    );

    if (!reviewTask) {
      throw new Error("No task with review status found");
    }

    return reviewTask.id;
  });

  // Open reviews panel first to mount the component
  await page.click('[data-testid="reviews-toggle"]');

  // Wait for panel to render and helper to be exposed
  await page.waitForSelector('[data-testid="reviews-panel"]', { timeout: 5000 });

  // Now use the helper to open the modal
  await page.evaluate((taskId) => {
    const openFn = (window as any).__openReviewDetailModal;
    if (!openFn) {
      throw new Error("__openReviewDetailModal not exposed on window");
    }
    openFn(taskId);
  }, taskId);

  // Wait for modal to appear
  await page.waitForSelector('[data-testid="review-detail-modal"]', { timeout: 5000 });
}

/**
 * Closes ReviewDetailModal by clicking the close button
 */
export async function closeReviewDetailModal(page: Page): Promise<void> {
  await page.click('[data-testid="review-detail-modal-close"]');
  await page.waitForSelector('[data-testid="review-detail-modal"]', {
    state: "hidden",
    timeout: 5000
  });
}
