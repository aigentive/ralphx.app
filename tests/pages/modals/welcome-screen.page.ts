/**
 * Page Object Model for WelcomeScreen component
 */

import { BasePage } from "../base.page";
import type { Page, Locator } from "@playwright/test";

export class WelcomeScreenPage extends BasePage {
  readonly container: Locator;
  readonly closeButton: Locator;
  readonly title: Locator;
  readonly tagline: Locator;
  readonly createProjectButton: Locator;
  readonly keyboardHint: Locator;
  readonly constellation: Locator;

  constructor(page: Page) {
    super(page);
    this.container = this.page.getByTestId("welcome-screen");
    this.closeButton = this.page.getByTestId("close-welcome-screen");
    this.title = this.container.locator("h1");
    this.tagline = this.container.locator("p").first();
    this.createProjectButton = this.page.getByTestId("create-first-project-button");
    this.keyboardHint = this.container.locator("kbd");
    this.constellation = this.container.locator(".absolute.inset-0.z-0").first();
  }

  async isVisible(): Promise<boolean> {
    return this.container.isVisible();
  }

  async clickCreateProject(): Promise<void> {
    await this.createProjectButton.click();
  }

  async close(): Promise<void> {
    await this.closeButton.click();
  }

  async getTitleText(): Promise<string> {
    return this.title.textContent() || "";
  }

  async getTaglineText(): Promise<string> {
    return this.tagline.textContent() || "";
  }

  async hasCloseButton(): Promise<boolean> {
    return this.closeButton.isVisible();
  }
}
