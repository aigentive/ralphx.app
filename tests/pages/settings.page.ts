import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

export class SettingsPage extends BasePage {
  readonly settingsView: Locator;
  readonly header: Locator;
  readonly settingsTitle: Locator;
  readonly savingIndicator: Locator;
  readonly errorBanner: Locator;

  // Execution Section
  readonly executionSection: Locator;
  readonly maxConcurrentTasksInput: Locator;
  readonly autoCommitToggle: Locator;
  readonly pauseOnFailureToggle: Locator;
  readonly reviewBeforeDestructiveToggle: Locator;

  // Model Section
  readonly modelSection: Locator;
  readonly modelSelect: Locator;
  readonly allowOpusUpgradeToggle: Locator;

  // Review Section
  readonly reviewSection: Locator;
  readonly aiReviewEnabledToggle: Locator;
  readonly aiReviewAutoFixToggle: Locator;
  readonly requireFixApprovalToggle: Locator;
  readonly requireHumanReviewToggle: Locator;
  readonly maxFixAttemptsInput: Locator;

  // Supervisor Section
  readonly supervisorSection: Locator;
  readonly supervisorEnabledToggle: Locator;
  readonly loopThresholdInput: Locator;
  readonly stuckTimeoutInput: Locator;

  constructor(page: Page) {
    super(page);

    // Main elements
    this.settingsView = page.locator('[data-testid="settings-view"]');
    this.header = page.locator('[data-testid="settings-view"]').locator("text=Settings").first().locator("..");
    this.settingsTitle = page.locator('[data-testid="settings-view"]').locator("h2:has-text('Settings')");
    this.savingIndicator = page.locator("text=Saving...");
    this.errorBanner = page.locator('[role="alert"]');

    // Execution Section
    this.executionSection = page.locator("text=Control task execution behavior and concurrency").locator("..");
    this.maxConcurrentTasksInput = page.locator('[data-testid="max-concurrent-tasks"]');
    this.autoCommitToggle = page.locator('[data-testid="auto-commit"]');
    this.pauseOnFailureToggle = page.locator('[data-testid="pause-on-failure"]');
    this.reviewBeforeDestructiveToggle = page.locator('[data-testid="review-before-destructive"]');

    // Model Section
    this.modelSection = page.locator("text=Configure AI model selection").locator("..");
    this.modelSelect = page.locator('[data-testid="model-selection"]');
    this.allowOpusUpgradeToggle = page.locator('[data-testid="allow-opus-upgrade"]');

    // Review Section
    this.reviewSection = page.locator("text=Configure code review automation").locator("..");
    this.aiReviewEnabledToggle = page.locator('[data-testid="ai-review-enabled"]');
    this.aiReviewAutoFixToggle = page.locator('[data-testid="ai-review-auto-fix"]');
    this.requireFixApprovalToggle = page.locator('[data-testid="require-fix-approval"]');
    this.requireHumanReviewToggle = page.locator('[data-testid="require-human-review"]');
    this.maxFixAttemptsInput = page.locator('[data-testid="max-fix-attempts"]');

    // Supervisor Section
    this.supervisorSection = page.locator("text=Configure watchdog monitoring for stuck or looping agents").locator("..");
    this.supervisorEnabledToggle = page.locator('[data-testid="supervisor-enabled"]');
    this.loopThresholdInput = page.locator('[data-testid="loop-threshold"]');
    this.stuckTimeoutInput = page.locator('[data-testid="stuck-timeout"]');
  }

  async waitForSettingsLoaded() {
    await this.settingsView.waitFor({ state: "visible" });
    await this.waitForAnimations();
  }

  async isToggleEnabled(toggle: Locator): Promise<boolean> {
    const state = await toggle.getAttribute("data-state");
    return state === "checked";
  }

  async getInputValue(input: Locator): Promise<string> {
    return (await input.inputValue()) || "";
  }
}
