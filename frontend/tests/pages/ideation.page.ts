import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

/**
 * Page object for the Ideation view.
 * Centralizes selectors and actions for ideation-related tests.
 */
export class IdeationPage extends BasePage {
  // Main view container
  readonly view: Locator;
  readonly header: Locator;
  readonly mainContent: Locator;

  // Session browser (left sidebar)
  readonly sessionBrowser: Locator;
  readonly sessionItem: (sessionId: string) => Locator;

  // Proposals panel (left split)
  readonly proposalsPanel: Locator;
  readonly importPlanButton: Locator;

  // Conversation panel (right split)
  readonly conversationPanel: Locator;

  // Resize handle
  readonly resizeHandle: Locator;

  // Navigation
  readonly navIdeation: Locator;

  constructor(page: Page) {
    super(page);

    // Main view
    this.view = page.locator('[data-testid="ideation-view"]');
    this.header = page.locator('[data-testid="ideation-header"]');
    this.mainContent = page.locator('[data-testid="ideation-main-content"]');

    // Session browser
    this.sessionBrowser = page.locator('[data-testid="session-browser"]');
    this.sessionItem = (sessionId) =>
      page.locator(`[data-testid="session-item-${sessionId}"]`);

    // Panels
    this.proposalsPanel = page.locator('[data-testid="proposals-panel"]');
    this.conversationPanel = page.locator('[data-testid="conversation-panel"]');
    this.importPlanButton = page.locator('[data-testid="import-plan-button"]');

    // Resize
    this.resizeHandle = page.locator('[data-testid="resize-handle"]');

    // Navigation
    this.navIdeation = page.locator('[data-testid="nav-ideation"]');
  }

  /**
   * Navigate to the ideation view by clicking the nav button
   */
  async navigateToIdeation() {
    await this.navIdeation.click();
    await this.view.waitFor({ timeout: 10000 });
  }

  /**
   * Wait for the ideation view to be fully loaded
   */
  async waitForIdeation() {
    await this.waitForApp();
    await this.view.waitFor({ timeout: 10000 });
  }

  /**
   * Check if there's an active session displayed
   */
  async hasActiveSession(): Promise<boolean> {
    return await this.header.isVisible();
  }
}
