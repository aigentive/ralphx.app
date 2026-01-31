import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

/**
 * Page object for the TaskDetailOverlay component.
 * This component is displayed in the Kanban split layout when a task is selected.
 *
 * The overlay has two main sections:
 * 1. Header (with testid prefix "task-overlay-") - title, category, buttons
 * 2. Content panel (TaskDetailPanel with prefix "task-detail-") - description, history, etc.
 */
export class TaskDetailPage extends BasePage {
  readonly taskDetailOverlay: Locator;
  readonly overlayBackdrop: Locator;

  // Header elements (in overlay)
  readonly overlayTitle: Locator;
  readonly overlayCategory: Locator;
  readonly overlayPriority: Locator;
  readonly overlayStatus: Locator;
  readonly closeButton: Locator;
  readonly editButton: Locator;
  readonly archiveButton: Locator;
  readonly restoreButton: Locator;
  readonly deleteButton: Locator;
  readonly ideationButton: Locator;

  // Content panel elements (inside TaskDetailPanel)
  readonly taskDetailPanel: Locator;
  readonly taskDescription: Locator;
  readonly reviewsSection: Locator;
  readonly historySection: Locator;
  readonly stepsSection: Locator;
  readonly contextSection: Locator;

  constructor(page: Page) {
    super(page);

    // Overlay container
    this.taskDetailOverlay = page.locator('[data-testid="task-detail-overlay"]');
    this.overlayBackdrop = page.locator('[data-testid="task-overlay-backdrop"]');

    // Header elements
    this.overlayTitle = page.locator('[data-testid="task-overlay-title"]');
    this.overlayCategory = page.locator('[data-testid="task-overlay-category"]');
    this.overlayPriority = page.locator('[data-testid="task-overlay-priority"]');
    this.overlayStatus = page.locator('[data-testid="task-overlay-status"]');
    this.closeButton = page.locator('[data-testid="task-overlay-close"]');
    this.editButton = page.locator('[data-testid="task-overlay-edit-button"]');
    this.archiveButton = page.locator('[data-testid="task-overlay-archive-button"]');
    this.restoreButton = page.locator('[data-testid="task-overlay-restore-button"]');
    this.deleteButton = page.locator('[data-testid="task-overlay-delete-button"]');
    this.ideationButton = page.locator('[data-testid="task-overlay-ideation-button"]');

    // Content panel elements
    this.taskDetailPanel = page.locator('[data-testid="task-detail-panel"]');
    this.taskDescription = page.locator('[data-testid="task-detail-description"]');
    this.reviewsSection = page.locator('[data-testid="task-detail-reviews-section"]');
    this.historySection = page.locator('[data-testid="task-detail-history-section"]');
    this.stepsSection = page.locator('[data-testid="task-detail-steps-section"]');
    this.contextSection = page.locator('[data-testid="task-context-section"]');
  }

  async openTaskDetail(taskId: string) {
    // Click on a task card to open the detail view
    const taskCard = this.page.locator(`[data-testid^="task-card-${taskId}"]`);
    await taskCard.click();

    // Wait for task detail overlay to be visible
    await this.taskDetailOverlay.waitFor({ state: "visible", timeout: 10000 });
  }

  async closeTaskDetail() {
    await this.closeButton.click();
    await this.taskDetailOverlay.waitFor({ state: "hidden", timeout: 5000 });
  }
}
