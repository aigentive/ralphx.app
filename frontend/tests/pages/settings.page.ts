import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

export type FontScaleOption = "default" | "lg" | "xl";

const FONT_SCALE_LABELS: Record<FontScaleOption, string> = {
  default: "Default (100%)",
  lg: "Large (110%)",
  xl: "Extra large (125%)",
};

export class SettingsPage extends BasePage {
  // Modal container (replaces old settings-view full-page shell)
  readonly settingsDialog: Locator;
  readonly settingsTitle: Locator;
  readonly closeButton: Locator;
  readonly savingIndicator: Locator;
  readonly errorBanner: Locator;

  // Accessibility Section — Font scale
  readonly fontScaleTrigger: Locator;

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

    // Main dialog element (modal overlay)
    this.settingsDialog = page.locator('[data-testid="settings-dialog"]');
    this.fontScaleTrigger = page.locator('[data-testid="font-scale"]');
    this.settingsTitle = this.settingsDialog.locator("text=Settings").first();
    this.closeButton = this.settingsDialog.getByRole("button", { name: "Close settings" });
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

  /** Open settings dialog by clicking the nav button */
  async openViaNavigation() {
    await this.page.click('[data-testid="nav-settings"]');
    await this.settingsDialog.waitFor({ state: "visible" });
  }

  /** Open settings dialog via uiStore.openModal (web-mode shortcut) */
  async openViaStore(section?: string) {
    await this.page.evaluate((sec) => {
      const uiStore = (window as unknown as { __uiStore?: { getState(): { openModal(type: string, ctx?: Record<string, unknown>): void } } }).__uiStore;
      if (uiStore) {
        uiStore.getState().openModal("settings", sec ? { section: sec } : undefined);
      }
    }, section);
    await this.settingsDialog.waitFor({ state: "visible" });
  }

  /** Open settings dialog via keyboard shortcut ⌘7 */
  async openViaKeyboard() {
    await this.page.keyboard.press("Meta+7");
    await this.settingsDialog.waitFor({ state: "visible" });
  }

  /** Select a section by its ID using the left-rail navigation */
  async selectSection(sectionId: string) {
    // Find the left-rail nav button whose data-section matches, or fall back to text-based nav
    const sectionButton = this.settingsDialog.getByRole("button", { name: new RegExp(`^${sectionId}$`, "i") });
    if (await sectionButton.isVisible()) {
      await sectionButton.click();
    } else {
      // Fallback: open via store with section deep-link
      await this.openViaStore(sectionId);
    }
  }

  /** Close the settings dialog via the close button */
  async closeModal() {
    await this.closeButton.click();
    await this.settingsDialog.waitFor({ state: "hidden" });
  }

  async waitForSettingsLoaded() {
    await this.settingsDialog.waitFor({ state: "visible" });
    await this.waitForAnimations();
  }

  async isToggleEnabled(toggle: Locator): Promise<boolean> {
    const state = await toggle.getAttribute("data-state");
    return state === "checked";
  }

  async getInputValue(input: Locator): Promise<string> {
    return (await input.inputValue()) || "";
  }

  /** Open Settings → Accessibility and select a font scale option. */
  async selectFontScale(scale: FontScaleOption) {
    await this.openViaStore("accessibility");
    await this.fontScaleTrigger.waitFor({ state: "visible" });
    await this.fontScaleTrigger.click();
    await this.page
      .locator('[role="option"]')
      .filter({ has: this.page.locator(`span:text-is("${FONT_SCALE_LABELS[scale]}")`) })
      .click();
    await this.waitForAnimations();
  }
}
