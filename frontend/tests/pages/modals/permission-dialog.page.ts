import { Page, Locator } from "@playwright/test";
import { BasePage } from "../base.page";

export class PermissionDialogPage extends BasePage {
  readonly dialog: Locator;
  readonly title: Locator;
  readonly toolName: Locator;
  readonly contextText: Locator;
  readonly inputPreview: Locator;
  readonly queueCount: Locator;
  readonly allowButton: Locator;
  readonly denyButton: Locator;
  readonly closeButton: Locator;

  constructor(page: Page) {
    super(page);
    this.dialog = page.getByRole("dialog");
    this.title = page.getByText("Permission Required");
    this.toolName = page.locator('[data-testid="permission-tool-name"]');
    this.contextText = page.locator('[data-testid="permission-context"]');
    this.inputPreview = page.locator('[data-testid="permission-input-preview"]');
    this.queueCount = page.locator('[data-testid="permission-queue-count"]');
    this.allowButton = page.getByRole("button", { name: /allow/i });
    this.denyButton = page.getByRole("button", { name: /deny/i });
    this.closeButton = page.getByRole("button", { name: /close/i });
  }

  async isVisible(): Promise<boolean> {
    return await this.dialog.isVisible();
  }

  async waitForDialog(): Promise<void> {
    await this.dialog.waitFor({ state: "visible", timeout: 5000 });
  }

  async waitForDialogToClose(): Promise<void> {
    await this.dialog.waitFor({ state: "hidden", timeout: 5000 });
  }

  async getToolName(): Promise<string> {
    return await this.toolName.textContent() || "";
  }

  async getContext(): Promise<string> {
    return await this.contextText.textContent() || "";
  }

  async getInputPreview(): Promise<string> {
    return await this.inputPreview.textContent() || "";
  }

  async getQueueCount(): Promise<string | null> {
    if (await this.queueCount.isVisible()) {
      return await this.queueCount.textContent();
    }
    return null;
  }

  async clickAllow(): Promise<void> {
    await this.allowButton.click();
  }

  async clickDeny(): Promise<void> {
    await this.denyButton.click();
  }

  async clickClose(): Promise<void> {
    await this.closeButton.click();
  }
}
