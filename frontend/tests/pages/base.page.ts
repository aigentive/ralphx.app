import { Page } from "@playwright/test";

export class BasePage {
  constructor(protected page: Page) {}

  async waitForApp() {
    await this.page.waitForSelector('[data-testid="app-header"]', {
      timeout: 10000,
    });
  }

  async waitForAnimations() {
    await this.page.waitForTimeout(500);
  }

  async navigateTo(path: string) {
    await this.page.goto(path);
    await this.waitForApp();
  }
}
