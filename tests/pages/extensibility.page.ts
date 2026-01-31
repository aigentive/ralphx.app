import { Page, Locator } from "@playwright/test";
import { BasePage } from "./base.page";

export class ExtensibilityPage extends BasePage {
  readonly extensibilityView: Locator;
  readonly tabNavigation: Locator;

  // Tabs
  readonly workflowsTab: Locator;
  readonly artifactsTab: Locator;
  readonly researchTab: Locator;
  readonly methodologiesTab: Locator;

  // Workflows Panel
  readonly workflowsPanel: Locator;
  readonly workflowCard: Locator;

  // Artifacts Panel
  readonly artifactsPanel: Locator;
  readonly bucketCard: Locator;

  // Research Panel
  readonly researchPanel: Locator;
  readonly researchProcessCard: Locator;

  // Methodologies Panel
  readonly methodologiesPanel: Locator;
  readonly methodologyCard: Locator;

  constructor(page: Page) {
    super(page);

    // Main view
    this.extensibilityView = page.locator('[data-testid="extensibility-view"]');
    this.tabNavigation = page.locator('[data-testid="tab-navigation"]');

    // Tabs
    this.workflowsTab = page.locator('[data-testid="tab-workflows"]');
    this.artifactsTab = page.locator('[data-testid="tab-artifacts"]');
    this.researchTab = page.locator('[data-testid="tab-research"]');
    this.methodologiesTab = page.locator('[data-testid="tab-methodologies"]');

    // Panel locators
    this.workflowsPanel = page.locator('[data-testid="workflows-panel"]');
    this.workflowCard = page.locator('[data-testid="workflow-card"]');

    this.artifactsPanel = page.locator('[data-testid="artifacts-panel"]');
    this.bucketCard = page.locator('[data-testid="bucket-card"]');

    this.researchPanel = page.locator('[data-testid="research-panel"]');
    this.researchProcessCard = page.locator('[data-testid="research-process-card"]');

    this.methodologiesPanel = page.locator('[data-testid="methodologies-panel"]');
    this.methodologyCard = page.locator('[data-testid="methodology-card"]');
  }

  async waitForExtensibilityLoaded() {
    await this.extensibilityView.waitFor({ state: "visible" });
    await this.waitForAnimations();
  }

  async switchTab(tab: "workflows" | "artifacts" | "research" | "methodologies") {
    const tabMap = {
      workflows: this.workflowsTab,
      artifacts: this.artifactsTab,
      research: this.researchTab,
      methodologies: this.methodologiesTab,
    };

    await tabMap[tab].click();
    await this.waitForAnimations();
  }

  async isTabActive(tab: "workflows" | "artifacts" | "research" | "methodologies"): Promise<boolean> {
    const tabMap = {
      workflows: this.workflowsTab,
      artifacts: this.artifactsTab,
      research: this.researchTab,
      methodologies: this.methodologiesTab,
    };

    const state = await tabMap[tab].getAttribute("data-state");
    return state === "active";
  }
}
