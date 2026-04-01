import { type Page, type Locator } from "@playwright/test";
import { BasePage } from "../base.page";

/**
 * Page Object for TaskFullView component
 *
 * TaskFullView is a full-screen overlay with split layout (details + chat)
 * Location: src/components/tasks/TaskFullView.tsx
 */
export class TaskFullViewPage extends BasePage {
  // Main container
  readonly container: Locator;
  readonly header: Locator;
  readonly leftPanel: Locator;
  readonly rightPanel: Locator;
  readonly dragHandle: Locator;
  readonly footer: Locator;

  // Header elements
  readonly backButton: Locator;
  readonly title: Locator;
  readonly priority: Locator;
  readonly status: Locator;
  readonly editButton: Locator;
  readonly archiveButton: Locator;
  readonly closeButton: Locator;

  // Footer execution controls
  readonly pauseButton: Locator;
  readonly stopButton: Locator;

  constructor(page: Page) {
    super(page);

    // Main sections
    this.container = page.getByTestId("task-fullview");
    this.header = page.getByTestId("task-fullview-header");
    this.leftPanel = page.getByTestId("task-fullview-left-panel");
    this.rightPanel = page.getByTestId("task-fullview-right-panel");
    this.dragHandle = page.getByTestId("task-fullview-drag-handle");
    this.footer = page.getByTestId("task-fullview-footer");

    // Header controls
    this.backButton = page.getByTestId("task-fullview-back-button");
    this.title = page.getByTestId("task-fullview-title");
    this.priority = page.getByTestId("task-fullview-priority");
    this.status = page.getByTestId("task-fullview-status");
    this.editButton = page.getByTestId("task-fullview-edit-button");
    this.archiveButton = page.getByTestId("task-fullview-archive-button");
    this.closeButton = page.getByTestId("task-fullview-close-button");

    // Footer controls
    this.pauseButton = page.getByTestId("task-fullview-pause-button");
    this.stopButton = page.getByTestId("task-fullview-stop-button");
  }

  /**
   * Check if TaskFullView is visible
   */
  async isVisible(): Promise<boolean> {
    return this.container.isVisible();
  }

  /**
   * Get task ID from container attribute
   */
  async getTaskId(): Promise<string | null> {
    return this.container.getAttribute("data-task-id");
  }

  /**
   * Get task title text
   */
  async getTitle(): Promise<string> {
    return this.title.textContent() || "";
  }

  /**
   * Get status text
   */
  async getStatus(): Promise<string> {
    return this.status.textContent() || "";
  }

  /**
   * Get priority text
   */
  async getPriority(): Promise<string> {
    return this.priority.textContent() || "";
  }

  /**
   * Check if footer execution controls are visible
   */
  async hasExecutionControls(): Promise<boolean> {
    return this.footer.isVisible();
  }

  /**
   * Close via back button
   */
  async clickBack(): Promise<void> {
    await this.backButton.click();
  }

  /**
   * Close via X button
   */
  async clickClose(): Promise<void> {
    await this.closeButton.click();
  }

  /**
   * Wait for view to be visible
   */
  async waitForVisible(): Promise<void> {
    await this.container.waitFor({ state: "visible" });
  }

  /**
   * Wait for view to be hidden
   */
  async waitForHidden(): Promise<void> {
    await this.container.waitFor({ state: "hidden" });
  }
}
