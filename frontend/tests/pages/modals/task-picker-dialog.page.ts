/**
 * Page Object: TaskPickerDialog
 *
 * Modal for selecting draft tasks to seed ideation sessions
 */

import { Locator, Page } from "@playwright/test";
import { BasePage } from "../base.page";

export class TaskPickerDialogPage extends BasePage {
  // Selectors
  readonly dialog: Locator;
  readonly title: Locator;
  readonly searchInput: Locator;
  readonly searchIcon: Locator;
  readonly taskList: Locator;
  readonly loadingState: Locator;
  readonly emptyState: Locator;
  readonly emptyStateIcon: Locator;
  readonly emptyStateMessage: Locator;

  constructor(page: Page) {
    super(page);
    this.dialog = page.locator('[role="dialog"]').filter({ hasText: "Select Draft Task" });
    this.title = this.dialog.locator("h2", { hasText: "Select Draft Task" });
    this.searchInput = this.dialog.locator('input[placeholder="Search tasks..."]');
    this.searchIcon = this.dialog.locator('svg').first(); // Search icon in input
    this.taskList = this.dialog.locator("div.space-y-1");
    this.loadingState = this.dialog.getByText("Loading tasks...");
    this.emptyState = this.dialog.locator("div.flex.flex-col.items-center.justify-center");
    this.emptyStateIcon = this.emptyState.locator("svg").first();
    this.emptyStateMessage = this.emptyState.locator("p.text-sm").first();
  }

  /**
   * Get task item by title
   */
  getTaskItem(title: string): Locator {
    return this.dialog.locator("button").filter({ hasText: title });
  }

  /**
   * Get all task items
   */
  getAllTaskItems(): Locator {
    return this.taskList.locator("button");
  }

  /**
   * Search for tasks
   */
  async search(query: string): Promise<void> {
    await this.searchInput.fill(query);
    await this.page.waitForTimeout(100); // Brief wait for filter to apply
  }

  /**
   * Select a task by title
   */
  async selectTask(title: string): Promise<void> {
    await this.getTaskItem(title).click();
  }

  /**
   * Check if dialog is visible
   */
  async isVisible(): Promise<boolean> {
    return await this.dialog.isVisible();
  }

  /**
   * Check if loading state is shown
   */
  async isLoading(): Promise<boolean> {
    return await this.loadingState.isVisible();
  }

  /**
   * Check if empty state is shown
   */
  async isEmpty(): Promise<boolean> {
    return await this.emptyState.isVisible();
  }

  /**
   * Get count of visible task items
   */
  async getTaskCount(): Promise<number> {
    const items = await this.getAllTaskItems().count();
    return items;
  }
}
