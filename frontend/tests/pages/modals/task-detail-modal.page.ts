import { Page, Locator } from "@playwright/test";
import { BasePage } from "../base.page";

/**
 * Page object for the task detail overlay visual surface.
 * The suite name is kept for continuity with existing snapshots.
 */
export class TaskDetailModalPage extends BasePage {
  readonly modal: Locator;
  readonly archivedBadge: Locator;
  readonly title: Locator;
  readonly category: Locator;
  readonly editButton: Locator;
  readonly archiveButton: Locator;
  readonly restoreButton: Locator;
  readonly deleteButton: Locator;
  readonly closeButton: Locator;
  readonly viewContextButton: Locator;
  readonly description: Locator;
  readonly reviewsSection: Locator;
  readonly reviewsLoading: Locator;
  readonly historySection: Locator;
  readonly contextSection: Locator;

  constructor(page: Page) {
    super(page);

    this.modal = page.locator('[data-testid="task-detail-overlay"]');
    this.archivedBadge = page.locator('[data-testid="archived-badge"]');
    this.title = page.locator('[data-testid="task-overlay-title"]');
    this.category = page.locator('[data-testid="task-overlay-category"]');
    this.editButton = page.locator('[data-testid="task-overlay-edit-button"]');
    this.archiveButton = page.locator('[data-testid="task-overlay-archive-button"]');
    this.restoreButton = page.locator('[data-testid="task-overlay-restore-button"]');
    this.deleteButton = page.locator('[data-testid="task-overlay-delete-button"]');
    this.closeButton = page.locator('[data-testid="task-overlay-close"]');
    this.viewContextButton = page.locator('[data-testid="view-context-button"]');
    this.description = page.locator('[data-testid="task-detail-description"]');
    this.reviewsSection = page.locator('[data-testid="task-detail-panel"]');
    this.reviewsLoading = page.locator('[data-testid="reviews-loading"]');
    this.historySection = page.locator('[data-testid="task-detail-history-section"]');
    this.contextSection = page.locator('[data-testid="task-context-section"]');
  }

  async isVisible(): Promise<boolean> {
    return await this.modal.isVisible();
  }

  async close(): Promise<void> {
    await this.closeButton.click();
  }

  async clickEdit(): Promise<void> {
    await this.editButton.click();
  }

  async clickArchive(): Promise<void> {
    await this.archiveButton.click();
  }

  async clickRestore(): Promise<void> {
    await this.restoreButton.click();
  }

  async clickDelete(): Promise<void> {
    await this.deleteButton.click();
  }

  async clickViewContext(): Promise<void> {
    await this.viewContextButton.click();
  }

  async getTitle(): Promise<string> {
    return await this.title.textContent() || "";
  }

  async getCategory(): Promise<string> {
    return await this.category.textContent() || "";
  }

  async getDescription(): Promise<string> {
    return await this.description.textContent() || "";
  }

  async isArchived(): Promise<boolean> {
    return await this.archivedBadge.isVisible();
  }

  async hasReviews(): Promise<boolean> {
    return await this.reviewsSection.isVisible();
  }

  async hasContext(): Promise<boolean> {
    return await this.contextSection.isVisible();
  }
}
