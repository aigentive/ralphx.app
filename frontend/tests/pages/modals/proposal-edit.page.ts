import { Page, Locator } from "@playwright/test";
import { BasePage } from "../base.page";

/**
 * Page object for the ProposalEditModal component.
 * Modal for editing task proposal details.
 */
export class ProposalEditModalPage extends BasePage {
  readonly modal: Locator;
  readonly title: Locator;
  readonly titleInput: Locator;
  readonly descriptionInput: Locator;
  readonly categorySelect: Locator;
  readonly prioritySelect: Locator;
  readonly stepInputs: Locator;
  readonly criterionInputs: Locator;
  readonly addStepButton: Locator;
  readonly addCriterionButton: Locator;
  readonly cancelButton: Locator;
  readonly confirmButton: Locator;

  constructor(page: Page) {
    super(page);

    this.modal = page.locator('[data-testid="proposal-edit-modal"]');
    this.title = page.locator("#modal-title");
    this.titleInput = page.locator("#proposal-title");
    this.descriptionInput = page.locator("#proposal-description");
    this.categorySelect = page.locator("#proposal-category");
    this.prioritySelect = page.locator("#proposal-priority");
    this.stepInputs = page.locator('[data-testid="step-input"]');
    this.criterionInputs = page.locator('[data-testid="criterion-input"]');
    this.addStepButton = page.getByRole("button", { name: /add step/i });
    this.addCriterionButton = page.getByRole("button", { name: /add criterion/i });
    this.cancelButton = page.locator('[data-testid="cancel-button"]');
    this.confirmButton = page.locator('[data-testid="confirm-button"]');
  }

  async waitForModal() {
    await this.modal.waitFor({ state: "visible", timeout: 5000 });
  }

  async fillTitle(title: string) {
    await this.titleInput.clear();
    await this.titleInput.fill(title);
  }

  async fillDescription(description: string) {
    await this.descriptionInput.clear();
    await this.descriptionInput.fill(description);
  }

  async selectCategory(category: string) {
    await this.categorySelect.selectOption(category);
  }

  async selectPriority(priority: string) {
    await this.prioritySelect.selectOption(priority);
  }

  async addStep(stepText: string) {
    await this.addStepButton.click();
    const steps = await this.stepInputs.all();
    const lastStep = steps[steps.length - 1];
    await lastStep.fill(stepText);
  }

  async addCriterion(criterionText: string) {
    await this.addCriterionButton.click();
    const criteria = await this.criterionInputs.all();
    const lastCriterion = criteria[criteria.length - 1];
    await lastCriterion.fill(criterionText);
  }

  async save() {
    await this.confirmButton.click();
    await this.modal.waitFor({ state: "hidden", timeout: 5000 });
  }

  async cancel() {
    await this.cancelButton.click();
    await this.modal.waitFor({ state: "hidden", timeout: 5000 });
  }

  async getStepCount(): Promise<number> {
    return await this.stepInputs.count();
  }

  async getCriterionCount(): Promise<number> {
    return await this.criterionInputs.count();
  }
}
