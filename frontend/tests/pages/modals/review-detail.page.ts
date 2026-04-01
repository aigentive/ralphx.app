/**
 * ReviewDetailModalPage - Page object for ReviewDetailModal
 *
 * Full-width modal showing task context, AI review summary, review history,
 * and DiffViewer with Approve/Request Changes actions
 */

import { Page, Locator } from "@playwright/test";
import { BasePage } from "../base.page";

export class ReviewDetailModalPage extends BasePage {
  // Modal container
  readonly modal: Locator;
  readonly modalTitle: Locator;
  readonly closeButton: Locator;

  // Left pane - Task details
  readonly taskTitle: Locator;
  readonly taskDescription: Locator;
  readonly aiReviewSummary: Locator;
  readonly reviewHistory: Locator;
  readonly revisionCountBadge: Locator;

  // Feedback input
  readonly feedbackInput: Locator;

  // Right pane - DiffViewer (delegated to DiffViewer component)
  readonly diffViewer: Locator;

  // Footer actions
  readonly requestChangesButton: Locator;
  readonly approveButton: Locator;

  constructor(page: Page) {
    super(page);

    // Modal
    this.modal = page.getByTestId("review-detail-modal");
    this.modalTitle = page.getByTestId("review-detail-modal-title");
    this.closeButton = page.getByTestId("review-detail-modal-close");

    // Left pane
    this.taskTitle = page.getByTestId("modal-task-title");
    this.taskDescription = page.getByTestId("modal-task-description");
    this.aiReviewSummary = page.getByTestId("ai-review-summary");
    this.reviewHistory = page.getByTestId("review-history");
    this.revisionCountBadge = page.getByTestId("revision-count-badge");

    // Feedback
    this.feedbackInput = page.getByTestId("feedback-input");

    // DiffViewer
    this.diffViewer = page.getByTestId("diff-viewer");

    // Actions
    this.requestChangesButton = page.getByTestId("review-detail-request-changes");
    this.approveButton = page.getByTestId("review-detail-approve");
  }

  /**
   * Check if modal is visible
   */
  async isVisible(): Promise<boolean> {
    return this.modal.isVisible();
  }

  /**
   * Get modal title text
   */
  async getTitle(): Promise<string> {
    return this.modalTitle.textContent() || "";
  }

  /**
   * Click approve button
   */
  async clickApprove(): Promise<void> {
    await this.approveButton.click();
  }

  /**
   * Click request changes button
   */
  async clickRequestChanges(): Promise<void> {
    await this.requestChangesButton.click();
  }

  /**
   * Enter feedback text
   */
  async enterFeedback(text: string): Promise<void> {
    await this.feedbackInput.fill(text);
  }

  /**
   * Close the modal
   */
  async close(): Promise<void> {
    await this.closeButton.click();
  }

  /**
   * Check if approve button is enabled
   */
  async isApproveEnabled(): Promise<boolean> {
    return this.approveButton.isEnabled();
  }

  /**
   * Check if request changes button is enabled
   */
  async isRequestChangesEnabled(): Promise<boolean> {
    return this.requestChangesButton.isEnabled();
  }
}
