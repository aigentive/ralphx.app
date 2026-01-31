import { Page, Locator } from "@playwright/test";
import { BasePage } from "../base.page";

/**
 * Page object for the ReviewsPanel component.
 * This is a slide-in panel that shows tasks awaiting review.
 */
export class ReviewsPanelPage extends BasePage {
  readonly reviewsToggle: Locator;
  readonly panel: Locator;
  readonly closeButton: Locator;
  readonly aiTab: Locator;
  readonly humanTab: Locator;
  readonly taskCards: Locator;
  readonly emptyState: Locator;
  readonly loadingSpinner: Locator;

  constructor(page: Page) {
    super(page);

    this.reviewsToggle = page.locator('[data-testid="reviews-toggle"]');
    this.panel = page.locator('[data-testid="reviews-panel"]');
    this.closeButton = page.locator('[data-testid="reviews-panel-close"]');
    this.aiTab = page.getByRole('tab', { name: /AI/ });
    this.humanTab = page.getByRole('tab', { name: /Human/ });
    this.taskCards = page.locator('[data-testid^="task-review-card-"]');
    this.emptyState = page.locator('[data-testid="reviews-panel-empty"]');
    this.loadingSpinner = page.locator('[data-testid="reviews-panel-loading"]');
  }

  async openPanel() {
    await this.reviewsToggle.click();
    await this.panel.waitFor({ state: "visible", timeout: 5000 });
  }

  async closePanel() {
    await this.closeButton.click();
    await this.panel.waitFor({ state: "hidden", timeout: 5000 });
  }

  async switchToAiTab() {
    await this.aiTab.click();
  }

  async switchToHumanTab() {
    await this.humanTab.click();
  }

  async getTaskCardCount(): Promise<number> {
    return await this.taskCards.count();
  }
}
