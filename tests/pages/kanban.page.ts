import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

/**
 * Page object for the Kanban Board view.
 * Centralizes selectors and actions for kanban-related tests.
 */
export class KanbanPage extends BasePage {
  // Board layout
  readonly board: Locator;

  // Column selectors (by status id)
  readonly column: (status: string) => Locator;
  readonly dropZone: (status: string) => Locator;

  // Task card selectors
  readonly taskCard: (id: string) => Locator;
  readonly taskCards: Locator;
  readonly taskTitle: (id: string) => Locator;

  // Header elements
  readonly branding: Locator;
  readonly chatToggle: Locator;
  readonly reviewsToggle: Locator;

  constructor(page: Page) {
    super(page);

    // Board layout
    this.board = page.locator('[data-testid="task-board"]');

    // Columns
    this.column = (status) => page.locator(`[data-testid="column-${status}"]`);
    this.dropZone = (status) =>
      page.locator(`[data-testid="drop-zone-${status}"]`);

    // Task cards
    this.taskCard = (id) => page.locator(`[data-testid="task-card-${id}"]`);
    this.taskCards = page.locator('[data-testid^="task-card-"]');
    this.taskTitle = (id) =>
      this.taskCard(id).locator('[data-testid="task-title"]');

    // Header elements
    this.branding = page.locator("text=RalphX");
    this.chatToggle = page.locator('[data-testid="chat-toggle"]');
    this.reviewsToggle = page.locator('[data-testid="reviews-toggle"]');
  }

  /**
   * Wait for the kanban board to be fully loaded with task cards
   */
  async waitForBoard() {
    await this.waitForApp();
    await this.taskCards.first().waitFor({ timeout: 10000 });
  }

  /**
   * Get the count of visible task cards
   */
  async getTaskCount(): Promise<number> {
    return await this.taskCards.count();
  }

  /**
   * Drag a task card to a different column
   */
  async dragTaskToColumn(taskId: string, targetStatus: string) {
    await this.taskCard(taskId).dragTo(this.dropZone(targetStatus));
  }
}
