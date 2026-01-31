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

  // Git mode selection
  readonly gitModeLocalRadio: Locator;
  readonly gitModeWorktreeRadio: Locator;

  // Worktree-specific fields (only visible when worktree mode selected)
  readonly baseBranchSelect: Locator;
  readonly worktreeBranchInput: Locator;
  readonly worktreePathInput: Locator;

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

    // Git mode radios
    this.gitModeLocalRadio = this.modal.locator('[data-testid="git-mode-local"]');
    this.gitModeWorktreeRadio = this.modal.locator('[data-testid="git-mode-worktree"]');

    // Worktree fields
    this.baseBranchSelect = this.modal.locator('[data-testid="base-branch-select"]');
    this.worktreeBranchInput = this.modal.locator('[data-testid="worktree-branch-input"]');
    this.worktreePathInput = this.modal.locator('[data-testid="worktree-path-display"]');

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
   * Select Local git mode
   */
  async selectLocalMode() {
    await this.gitModeLocalRadio.click();
  }

  /**
   * Select Worktree git mode
   */
  async selectWorktreeMode() {
    await this.gitModeWorktreeRadio.click();
  }

  /**
   * Select a base branch (only available in worktree mode)
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
   * Fill in the worktree branch name (only available in worktree mode)
   */
  async fillWorktreeBranch(branch: string) {
    await this.worktreeBranchInput.fill(branch);
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
   * Check if worktree fields are visible
   */
  async areWorktreeFieldsVisible(): Promise<boolean> {
    return this.baseBranchSelect.isVisible();
  }
}
