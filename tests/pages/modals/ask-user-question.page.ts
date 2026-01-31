import { Page, Locator } from "@playwright/test";
import { BasePage } from "../base.page";

/**
 * Page object for the AskUserQuestionModal component.
 * This modal allows agents to ask questions requiring user input during execution.
 */
export class AskUserQuestionModalPage extends BasePage {
  readonly modal: Locator;
  readonly header: Locator;
  readonly questionText: Locator;
  readonly submitButton: Locator;
  readonly otherInput: Locator;

  constructor(page: Page) {
    super(page);

    this.modal = page.locator('[data-testid="ask-user-question-modal"]');
    this.header = page.locator('[data-testid="question-header"]');
    this.questionText = page.locator('[data-testid="question-text"]');
    this.submitButton = page.getByRole("button", { name: /submit/i });
    this.otherInput = page.locator('[data-testid="other-input"]');
  }

  /**
   * Get a radio button by its label (for single-select questions)
   */
  getRadioByLabel(label: string): Locator {
    return this.page.getByRole("radio", { name: new RegExp(label, "i") });
  }

  /**
   * Get a checkbox by its label (for multi-select questions)
   */
  getCheckboxByLabel(label: string): Locator {
    return this.page.getByRole("checkbox", { name: new RegExp(label, "i") });
  }

  /**
   * Get the "Other" radio button (single-select)
   */
  getOtherRadio(): Locator {
    return this.page.getByRole("radio", { name: /other/i });
  }

  /**
   * Get the "Other" checkbox (multi-select)
   */
  getOtherCheckbox(): Locator {
    return this.page.getByRole("checkbox", { name: /other/i });
  }

  /**
   * Select a single option by label
   */
  async selectOption(label: string) {
    const radio = this.getRadioByLabel(label);
    await radio.click();
  }

  /**
   * Select the "Other" option and enter custom text
   */
  async selectOtherWithText(text: string) {
    const otherRadio = this.getOtherRadio();
    await otherRadio.click();
    await this.otherInput.waitFor({ state: "visible" });
    await this.otherInput.fill(text);
  }

  /**
   * Toggle a checkbox option (for multi-select)
   */
  async toggleCheckbox(label: string) {
    const checkbox = this.getCheckboxByLabel(label);
    await checkbox.click();
  }

  /**
   * Select multiple checkbox options
   */
  async selectMultiple(labels: string[]) {
    for (const label of labels) {
      await this.toggleCheckbox(label);
    }
  }

  /**
   * Submit the answer
   */
  async submit() {
    await this.submitButton.click();
  }

  /**
   * Check if modal is visible
   */
  async isVisible(): Promise<boolean> {
    return await this.modal.isVisible();
  }

  /**
   * Wait for modal to appear
   */
  async waitForModal() {
    await this.modal.waitFor({ state: "visible", timeout: 5000 });
  }

  /**
   * Wait for modal to disappear (after submit)
   */
  async waitForModalClose() {
    await this.modal.waitFor({ state: "hidden", timeout: 5000 });
  }

  /**
   * Get the count of radio buttons (for single-select questions)
   */
  async getRadioCount(): Promise<number> {
    return await this.page.getByRole("radio").count();
  }

  /**
   * Get the count of checkboxes (for multi-select questions)
   */
  async getCheckboxCount(): Promise<number> {
    return await this.page.getByRole("checkbox").count();
  }
}
