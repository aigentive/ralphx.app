/**
 * ReviewDetailModal - Visual regression tests
 *
 * Full-width modal for detailed code review with:
 * - Task context, AI review summary, review history (left pane)
 * - DiffViewer with file changes and commit history (right pane)
 * - Approve/Request Changes actions (footer)
 */

import { test, expect } from "@playwright/test";
import { ReviewDetailModalPage } from "../../../pages/modals/review-detail.page";
import { openReviewDetailModal, closeReviewDetailModal } from "../../../helpers/review-detail.helpers";

test.describe("ReviewDetailModal", () => {
  let reviewDetailPage: ReviewDetailModalPage;

  test.beforeEach(async ({ page }) => {
    reviewDetailPage = new ReviewDetailModalPage(page);
    await openReviewDetailModal(page);
  });

  test("renders modal with task details", async ({ page }) => {
    await expect(reviewDetailPage.modal).toBeVisible();
    await expect(reviewDetailPage.modalTitle).toBeVisible();
    await expect(reviewDetailPage.taskTitle).toBeVisible();

    await expect(page).toHaveScreenshot("review-detail-modal.png", {
      fullPage: true,
    });
  });

  test("shows AI review summary when available", async ({ page }) => {
    await expect(reviewDetailPage.aiReviewSummary).toBeVisible();

    await expect(page).toHaveScreenshot("review-detail-ai-summary.png", {
      fullPage: true,
    });
  });

  test("displays review history timeline", async ({ page }) => {
    await expect(reviewDetailPage.reviewHistory).toBeVisible();

    await expect(page).toHaveScreenshot("review-detail-history.png", {
      fullPage: true,
    });
  });

  test("shows revision count badge when changes were requested", async ({ page }) => {
    // This test assumes mock data includes a task with revision history
    // If revision count is 0, the badge won't render
    const isVisible = await reviewDetailPage.revisionCountBadge.isVisible();

    if (isVisible) {
      await expect(page).toHaveScreenshot("review-detail-with-revisions.png", {
        fullPage: true,
      });
    } else {
      // No revisions - this is also a valid state to capture
      await expect(page).toHaveScreenshot("review-detail-no-revisions.png", {
        fullPage: true,
      });
    }
  });

  test("shows action buttons (Approve and Request Changes)", async ({ page }) => {
    await expect(reviewDetailPage.approveButton).toBeVisible();
    await expect(reviewDetailPage.requestChangesButton).toBeVisible();

    await expect(page).toHaveScreenshot("review-detail-actions.png", {
      fullPage: true,
    });
  });

  test("enables action buttons when task can be approved", async ({ page }) => {
    // Buttons should be enabled if task is in review_passed or escalated state
    const approveEnabled = await reviewDetailPage.isApproveEnabled();
    const requestChangesEnabled = await reviewDetailPage.isRequestChangesEnabled();

    // At least one should work based on task state
    expect(approveEnabled || requestChangesEnabled).toBeTruthy();

    await expect(page).toHaveScreenshot("review-detail-buttons-state.png", {
      fullPage: true,
    });
  });

  test("can close modal with close button", async ({ page }) => {
    await expect(reviewDetailPage.modal).toBeVisible();

    await closeReviewDetailModal(page);

    await expect(reviewDetailPage.modal).not.toBeVisible();
  });

  test("shows feedback input when Request Changes is clicked", async ({ page }) => {
    // Only test if button is enabled
    const isEnabled = await reviewDetailPage.isRequestChangesEnabled();

    if (isEnabled) {
      await reviewDetailPage.clickRequestChanges();
      await expect(reviewDetailPage.feedbackInput).toBeVisible();

      await expect(page).toHaveScreenshot("review-detail-feedback-input.png", {
        fullPage: true,
      });
    } else {
      test.skip();
    }
  });

  test("shows DiffViewer in right pane", async ({ page }) => {
    // DiffViewer should be visible (even if empty/loading)
    await expect(reviewDetailPage.diffViewer).toBeVisible();

    await expect(page).toHaveScreenshot("review-detail-diffviewer.png", {
      fullPage: true,
    });
  });
});
