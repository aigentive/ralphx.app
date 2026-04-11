import { Page, Locator } from "@playwright/test";
import { BasePage } from "../base.page";

/**
 * Page Object for ProjectCreationWizard modal
 */
export class ProjectCreationWizardPage extends BasePage {
  // Modal container
  readonly modal: Locator;

  // Header elements
  readonly title: Locator;

  // Form fields
  readonly projectNameInput: Locator;
  readonly workingDirectoryInput: Locator;
  readonly browseFolderButton: Locator;

  // Worktree-first fields
  readonly baseBranchSelect: Locator;
  readonly worktreePathInput: Locator;
  readonly advancedSettingsTrigger: Locator;
  readonly worktreeParentInput: Locator;

  // Action buttons
  readonly createButton: Locator;
  readonly cancelButton: Locator;

  // Error message
  readonly errorMessage: Locator;

  constructor(page: Page) {
    super(page);

    this.modal = page.locator('[data-testid="project-creation-wizard"]');
    this.title = this.modal.getByRole("heading", { name: "Create New Project" });

    // Form fields
    this.projectNameInput = this.modal.locator('[data-testid="project-name-input"]');
    this.workingDirectoryInput = this.modal.locator('[data-testid="folder-input"]');
    this.browseFolderButton = this.modal.locator('[data-testid="browse-button"]');

    // Worktree fields
    this.baseBranchSelect = this.modal.locator('[data-testid="base-branch-select"]');
    this.worktreePathInput = this.modal.locator('[data-testid="worktree-path-display"]');
    this.advancedSettingsTrigger = this.modal.locator('[data-testid="advanced-settings-trigger"]');
    this.worktreeParentInput = this.modal.locator('[data-testid="worktree-parent-input"]');

    // Buttons
    this.createButton = this.modal.locator('[data-testid="create-button"]');
    this.cancelButton = this.modal.locator('[data-testid="cancel-button"]');

    // Error
    this.errorMessage = this.modal.locator('[data-testid="wizard-error"]');
  }

  /**
   * Wait for the modal to be visible
   */
  async waitForModal() {
    await this.modal.waitFor({ state: "visible", timeout: 5000 });
  }

  /**
   * Fill in the project name
   */
  async fillProjectName(name: string) {
    await this.projectNameInput.fill(name);
  }

  /**
   * Click the browse folder button
   */
  async clickBrowseFolder() {
    await this.browseFolderButton.click();
  }

  /**
   * Expand advanced settings
   */
  async openAdvancedSettings() {
    await this.advancedSettingsTrigger.click();
  }

  /**
   * Select a base branch
   */
  async selectBaseBranch(branch: string) {
    // Click the select trigger
    await this.baseBranchSelect.click();

    // Wait for the select dropdown to appear (it's rendered in a portal)
    const option = this.page.locator(`[role="option"]`).filter({ hasText: branch });
    await option.waitFor({ state: "visible", timeout: 5000 });
    await option.click();
  }

  /**
   * Click the Create button
   */
  async clickCreate() {
    await this.createButton.click();
  }

  /**
   * Click the Cancel button
   */
  async clickCancel() {
    await this.cancelButton.click();
  }

  /**
   * Get the current working directory value
   */
  async getWorkingDirectory(): Promise<string> {
    return this.workingDirectoryInput.inputValue();
  }

  /**
   * Check if base branch and worktree path are visible
   */
  async areWorktreeFieldsVisible(): Promise<boolean> {
    return (await this.baseBranchSelect.isVisible()) && (await this.worktreePathInput.isVisible());
  }
}
