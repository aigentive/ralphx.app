import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

/**
 * Page object for error states across the application
 */
export class ErrorStatePage extends BasePage {
  // Error Boundary selectors
  readonly errorContainer: Locator;
  readonly errorTitle: Locator;
  readonly errorMessage: Locator;
  readonly errorStack: Locator;
  readonly tryAgainButton: Locator;
  readonly componentStackDetails: Locator;

  constructor(page: Page) {
    super(page);

    // Error boundary elements (based on ErrorBoundary.tsx)
    this.errorContainer = page.locator("div").filter({
      hasText: "Something went wrong",
    });
    this.errorTitle = page.locator("h2").filter({
      hasText: "Something went wrong",
    });
    this.errorMessage = this.errorContainer.locator("code").first();
    this.tryAgainButton = this.errorContainer.locator(
      'button:has-text("Try Again")'
    );
    this.componentStackDetails = this.errorContainer.locator("details");
    this.errorStack = this.componentStackDetails.locator("pre");
  }

  /**
   * Verify error boundary is displayed
   */
  async isErrorBoundaryVisible(): Promise<boolean> {
    return this.errorContainer.isVisible();
  }

  /**
   * Get the error message text
   */
  async getErrorMessage(): Promise<string> {
    return this.errorMessage.textContent().then((text) => text || "");
  }

  /**
   * Click the Try Again button
   */
  async clickTryAgain() {
    await this.tryAgainButton.click();
  }

  /**
   * Expand component stack details
   */
  async expandComponentStack() {
    await this.componentStackDetails.click();
  }

  /**
   * Wait for error boundary to appear
   */
  async waitForErrorBoundary(timeout: number = 5000) {
    await this.errorContainer.waitFor({ state: "visible", timeout });
  }
}
