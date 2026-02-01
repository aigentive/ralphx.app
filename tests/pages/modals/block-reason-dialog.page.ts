import { Page } from "@playwright/test";
import { BasePage } from "../base.page";

/**
 * Page Object for BlockReasonDialog
 * Modal for capturing an optional reason when blocking a task
 */
export class BlockReasonDialogPage extends BasePage {
  constructor(page: Page) {
    super(page);
  }

  // ============================================================================
  // Locators
  // ============================================================================

  get dialog() {
    return this.page.getByTestId("block-reason-dialog");
  }

  get title() {
    return this.page.getByTestId("dialog-title");
  }

  get reasonInput() {
    return this.page.getByTestId("block-reason-input");
  }

  get cancelButton() {
    return this.page.getByTestId("cancel-button");
  }

  get confirmButton() {
    return this.page.getByTestId("confirm-button");
  }

  // ============================================================================
  // Actions
  // ============================================================================

  async enterReason(reason: string) {
    await this.reasonInput.fill(reason);
  }

  async clickCancel() {
    await this.cancelButton.click();
  }

  async clickConfirm() {
    await this.confirmButton.click();
  }

  async confirmWithKeyboard() {
    await this.reasonInput.press("Meta+Enter");
  }

  // ============================================================================
  // Assertions
  // ============================================================================

  async expectVisible() {
    await this.dialog.waitFor({ state: "visible" });
  }

  async expectHidden() {
    await this.dialog.waitFor({ state: "hidden" });
  }

  async expectReasonValue(value: string) {
    await this.page.waitForTimeout(100); // Small wait for value to update
    const actualValue = await this.reasonInput.inputValue();
    if (actualValue !== value) {
      throw new Error(`Expected reason value to be "${value}", but got "${actualValue}"`);
    }
  }
}
