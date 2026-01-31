import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

/**
 * Page object for the Activity view.
 * Centralizes selectors and actions for activity-related tests.
 */
export class ActivityPage extends BasePage {
  // Main view container
  readonly view: Locator;

  // Header elements
  readonly clearButton: Locator;
  readonly alertBadge: Locator;

  // Search and filters
  readonly searchBar: Locator;
  readonly viewModeToggle: Locator;
  readonly allFilterTab: Locator;
  readonly thinkingFilterTab: Locator;
  readonly toolCallsFilterTab: Locator;
  readonly statusFilter: Locator;
  readonly roleFilter: Locator;

  // Messages container
  readonly messagesContainer: Locator;
  readonly activityMessage: (index: number) => Locator;

  // Empty state
  readonly emptyState: Locator;

  // Scroll controls
  readonly scrollToBottomButton: Locator;

  // Navigation
  readonly navActivity: Locator;

  constructor(page: Page) {
    super(page);

    // Main view
    this.view = page.locator('[data-testid="activity-view"]');

    // Header elements
    this.clearButton = page.locator('[data-testid="activity-clear"]');
    this.alertBadge = this.view.locator('span:has-text("alert")');

    // Search and filters
    this.searchBar = this.view.locator('input[type="text"]');
    this.viewModeToggle = this.view.locator('button:has-text("Realtime"), button:has-text("Historical")');
    this.allFilterTab = page.getByRole('tab', { name: 'All', exact: true });
    this.thinkingFilterTab = page.getByRole('tab', { name: 'Thinking' });
    this.toolCallsFilterTab = page.getByRole('tab', { name: 'Tool Calls' });
    this.statusFilter = this.view.locator('button:has-text("Status")');
    this.roleFilter = this.view.locator('button:has-text("Role")');

    // Messages
    this.messagesContainer = page.locator('[data-testid="activity-messages"]');
    this.activityMessage = (index) => this.messagesContainer.locator('[data-testid^="activity-message-"]').nth(index);

    // Empty state
    this.emptyState = this.messagesContainer.locator('text=No activity yet');

    // Scroll controls
    this.scrollToBottomButton = page.locator('[data-testid="activity-scroll-to-bottom"]');

    // Navigation
    this.navActivity = page.locator('[data-testid="nav-activity"]');
  }

  /**
   * Navigate to the activity view by clicking the nav button
   */
  async navigateToActivity() {
    await this.navActivity.click();
    await this.view.waitFor({ timeout: 10000 });
  }

  /**
   * Wait for the activity view to be fully loaded
   */
  async waitForActivity() {
    await this.waitForApp();
    await this.view.waitFor({ timeout: 10000 });
  }

  /**
   * Check if any messages are displayed
   */
  async hasMessages(): Promise<boolean> {
    const count = await this.messagesContainer.locator('[data-testid^="activity-message-"]').count();
    return count > 0;
  }

  /**
   * Get the count of visible activity messages
   */
  async getMessageCount(): Promise<number> {
    return await this.messagesContainer.locator('[data-testid^="activity-message-"]').count();
  }

  /**
   * Clear all activity messages
   */
  async clearMessages() {
    await this.clearButton.click();
  }

  /**
   * Search for messages
   */
  async searchMessages(query: string) {
    await this.searchBar.fill(query);
  }
}
