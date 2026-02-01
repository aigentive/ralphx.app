/**
 * Page Object Model for MergeWorkflowDialog component
 */

import { BasePage } from "../base.page";
import type { Page, Locator } from "@playwright/test";

export class MergeWorkflowDialogPage extends BasePage {
  readonly dialog: Locator;
  readonly title: Locator;
  readonly commitCount: Locator;
  readonly branchName: Locator;
  readonly viewDiffButton: Locator;
  readonly viewCommitsButton: Locator;
  readonly cancelButton: Locator;
  readonly confirmButton: Locator;
  readonly errorMessage: Locator;
  readonly discardConfirmation: Locator;

  constructor(page: Page) {
    super(page);
    this.dialog = this.page.getByTestId("merge-workflow-dialog");
    this.title = this.dialog.getByRole("heading");
    this.commitCount = this.page.getByTestId("commit-count");
    this.branchName = this.page.getByTestId("branch-name");
    this.viewDiffButton = this.page.getByTestId("view-diff-button");
    this.viewCommitsButton = this.page.getByTestId("view-commits-button");
    this.cancelButton = this.page.getByTestId("cancel-button");
    this.confirmButton = this.page.getByTestId("confirm-button");
    this.errorMessage = this.page.getByTestId("dialog-error");
    this.discardConfirmation = this.page.getByTestId("discard-confirmation");
  }

  async isVisible(): Promise<boolean> {
    return this.dialog.isVisible();
  }

  async getTitleText(): Promise<string> {
    return (await this.title.textContent()) || "";
  }

  async getCommitCountText(): Promise<string> {
    return (await this.commitCount.textContent()) || "";
  }

  async getBranchNameText(): Promise<string> {
    return (await this.branchName.textContent()) || "";
  }

  async selectOption(option: "merge" | "rebase" | "create_pr" | "keep_worktree" | "discard"): Promise<void> {
    const optionLabel = this.page.getByTestId(`merge-option-${option}`);
    await optionLabel.click();
  }

  async isOptionSelected(option: "merge" | "rebase" | "create_pr" | "keep_worktree" | "discard"): Promise<boolean> {
    const optionLabel = this.page.getByTestId(`merge-option-${option}`);
    const selected = await optionLabel.getAttribute("data-selected");
    return selected === "true";
  }

  async clickViewDiff(): Promise<void> {
    await this.viewDiffButton.click();
  }

  async clickViewCommits(): Promise<void> {
    await this.viewCommitsButton.click();
  }

  async clickCancel(): Promise<void> {
    await this.cancelButton.click();
  }

  async clickConfirm(): Promise<void> {
    await this.confirmButton.click();
  }

  async hasViewDiffButton(): Promise<boolean> {
    return this.viewDiffButton.isVisible();
  }

  async hasViewCommitsButton(): Promise<boolean> {
    return this.viewCommitsButton.isVisible();
  }

  async hasError(): Promise<boolean> {
    return this.errorMessage.isVisible();
  }

  async getErrorText(): Promise<string> {
    return (await this.errorMessage.textContent()) || "";
  }

  async hasDiscardConfirmation(): Promise<boolean> {
    return this.discardConfirmation.isVisible();
  }

  async getConfirmButtonText(): Promise<string> {
    return (await this.confirmButton.textContent()) || "";
  }

  async isConfirmButtonDisabled(): Promise<boolean> {
    return (await this.confirmButton.getAttribute("disabled")) !== null;
  }

  async isCancelButtonDisabled(): Promise<boolean> {
    return (await this.cancelButton.getAttribute("disabled")) !== null;
  }
}
