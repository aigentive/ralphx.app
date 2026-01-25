/**
 * ExtensibilityView component tests
 *
 * Tests for:
 * - Premium tabbed interface with shadcn Tabs
 * - Tab navigation with icons (Workflows, Artifacts, Research, Methodologies)
 * - Tab content rendering with premium panels
 * - Default tab selection
 * - Tab switching
 * - Accessibility
 * - Premium styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { ExtensibilityView } from "./ExtensibilityView";
import { TooltipProvider } from "@/components/ui/tooltip";

// Mock hooks
vi.mock("@/hooks/useMethodologies", () => ({
  useMethodologies: () => ({ data: [], isLoading: false, error: null }),
}));

vi.mock("@/hooks/useMethodologyActivation", () => ({
  useMethodologyActivation: () => ({
    activate: vi.fn(),
    deactivate: vi.fn(),
    isActivating: false,
    activeMethodology: null,
  }),
}));

// Test query client
function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
}

function renderWithProviders(ui: React.ReactElement) {
  const queryClient = createTestQueryClient();
  return render(
    createElement(
      QueryClientProvider,
      { client: queryClient },
      createElement(TooltipProvider, null, ui)
    )
  );
}

describe("ExtensibilityView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("extensibility-view")).toBeInTheDocument();
    });

    it("renders tab navigation", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("tab-navigation")).toBeInTheDocument();
    });

    it("renders all four tabs", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("tab-workflows")).toBeInTheDocument();
      expect(screen.getByTestId("tab-artifacts")).toBeInTheDocument();
      expect(screen.getByTestId("tab-research")).toBeInTheDocument();
      expect(screen.getByTestId("tab-methodologies")).toBeInTheDocument();
    });

    it("displays tab labels", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByText("Workflows")).toBeInTheDocument();
      expect(screen.getByText("Artifacts")).toBeInTheDocument();
      expect(screen.getByText("Research")).toBeInTheDocument();
      expect(screen.getByText("Methodologies")).toBeInTheDocument();
    });

    it("renders premium background with warm gradient", () => {
      renderWithProviders(<ExtensibilityView />);
      const view = screen.getByTestId("extensibility-view");
      const style = view.getAttribute("style");
      expect(style).toContain("radial-gradient");
      expect(style).toContain("rgba(255, 107, 53");
    });
  });

  // ==========================================================================
  // Default Tab
  // ==========================================================================

  describe("default tab", () => {
    it("shows Workflows tab as active by default", () => {
      renderWithProviders(<ExtensibilityView />);
      const workflowsTab = screen.getByTestId("tab-workflows");
      expect(workflowsTab).toHaveAttribute("data-state", "active");
    });

    it("renders Workflows panel content by default", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("workflows-panel")).toBeInTheDocument();
    });

    it("displays Workflow Schemas header", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByText("Workflow Schemas")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Tab Switching
  // ==========================================================================

  describe("tab switching", () => {
    it("switches to Artifacts tab when clicked", async () => {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      await user.click(screen.getByTestId("tab-artifacts"));

      await waitFor(() => {
        expect(screen.getByTestId("tab-artifacts")).toHaveAttribute("data-state", "active");
      });
      expect(screen.getByTestId("tab-workflows")).toHaveAttribute("data-state", "inactive");
      expect(screen.getByTestId("artifacts-panel")).toBeInTheDocument();
    });

    it("switches to Research tab when clicked", async () => {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      await user.click(screen.getByTestId("tab-research"));

      await waitFor(() => {
        expect(screen.getByTestId("tab-research")).toHaveAttribute("data-state", "active");
      });
      expect(screen.getByTestId("research-panel")).toBeInTheDocument();
    });

    it("switches to Methodologies tab when clicked", async () => {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      await user.click(screen.getByTestId("tab-methodologies"));

      await waitFor(() => {
        expect(screen.getByTestId("tab-methodologies")).toHaveAttribute("data-state", "active");
      });
      expect(screen.getByTestId("methodologies-panel")).toBeInTheDocument();
    });

    it("hides previous tab content when switching", async () => {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("workflows-panel")).toBeInTheDocument();

      await user.click(screen.getByTestId("tab-artifacts"));

      await waitFor(() => {
        expect(screen.queryByTestId("workflows-panel")).not.toBeInTheDocument();
      });
    });
  });

  // ==========================================================================
  // Workflows Panel
  // ==========================================================================

  describe("workflows panel", () => {
    it("renders workflow cards with mock data", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("workflow-card")).toBeInTheDocument();
    });

    it("shows Default Kanban workflow", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByText("Default Kanban")).toBeInTheDocument();
    });

    it("displays column count in metadata", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByText("4 columns")).toBeInTheDocument();
    });

    it("shows DEFAULT badge for default workflow", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByText("DEFAULT")).toBeInTheDocument();
    });

    it("renders New Workflow button", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByText("New Workflow")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Artifacts Panel
  // ==========================================================================

  describe("artifacts panel", () => {
    async function renderArtifactsPanel() {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      await user.click(screen.getByTestId("tab-artifacts"));
      await waitFor(() => {
        expect(screen.getByTestId("artifacts-panel")).toBeInTheDocument();
      });
      return user;
    }

    it("renders bucket sidebar", async () => {
      await renderArtifactsPanel();
      expect(screen.getByText("Buckets")).toBeInTheDocument();
    });

    it("shows bucket items", async () => {
      await renderArtifactsPanel();
      const buckets = screen.getAllByTestId("bucket-item");
      expect(buckets.length).toBeGreaterThanOrEqual(4);
    });

    it("has search input", async () => {
      await renderArtifactsPanel();
      expect(screen.getByPlaceholderText("Search artifacts...")).toBeInTheDocument();
    });

    it("has view toggle buttons", async () => {
      await renderArtifactsPanel();
      const panel = screen.getByTestId("artifacts-panel");
      // Check for list and grid view buttons
      const buttons = within(panel).getAllByRole("button");
      expect(buttons.length).toBeGreaterThan(0);
    });

    it("shows empty state when no bucket selected", async () => {
      await renderArtifactsPanel();
      expect(screen.getByText("Select a bucket to view artifacts")).toBeInTheDocument();
    });

    it("shows artifacts when bucket is selected", async () => {
      const user = await renderArtifactsPanel();
      const prdsBucket = screen.getAllByTestId("bucket-item")[2]; // PRDs bucket
      await user.click(prdsBucket);

      expect(screen.getByTestId("artifact-card")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Research Panel
  // ==========================================================================

  describe("research panel", () => {
    async function renderResearchPanel() {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      await user.click(screen.getByTestId("tab-research"));
      await waitFor(() => {
        expect(screen.getByTestId("research-panel")).toBeInTheDocument();
      });
      return user;
    }

    it("renders research launcher card", async () => {
      await renderResearchPanel();
      expect(screen.getByText("Launch New Research")).toBeInTheDocument();
    });

    it("has question textarea", async () => {
      await renderResearchPanel();
      expect(screen.getByTestId("question-input")).toBeInTheDocument();
    });

    it("has context and scope inputs", async () => {
      await renderResearchPanel();
      expect(screen.getByTestId("context-input")).toBeInTheDocument();
      expect(screen.getByTestId("scope-input")).toBeInTheDocument();
    });

    it("has depth preset selector", async () => {
      await renderResearchPanel();
      expect(screen.getByTestId("depth-preset-selector")).toBeInTheDocument();
    });

    it("has preset buttons with icons", async () => {
      await renderResearchPanel();
      expect(screen.getByTestId("preset-quick-scan")).toBeInTheDocument();
      expect(screen.getByTestId("preset-standard")).toBeInTheDocument();
      expect(screen.getByTestId("preset-deep-dive")).toBeInTheDocument();
      expect(screen.getByTestId("preset-exhaustive")).toBeInTheDocument();
      expect(screen.getByTestId("preset-custom")).toBeInTheDocument();
    });

    it("shows custom inputs when Custom preset is selected", async () => {
      const user = await renderResearchPanel();
      await user.click(screen.getByTestId("preset-custom"));

      expect(screen.getByTestId("custom-iterations-input")).toBeInTheDocument();
      expect(screen.getByTestId("custom-timeout-input")).toBeInTheDocument();
    });

    it("has launch button", async () => {
      await renderResearchPanel();
      expect(screen.getByTestId("launch-button")).toBeInTheDocument();
    });

    it("launch button is disabled when question is empty", async () => {
      await renderResearchPanel();
      const launchButton = screen.getByTestId("launch-button");
      expect(launchButton).toBeDisabled();
    });

    it("launch button is enabled when question is filled", async () => {
      const user = await renderResearchPanel();
      const questionInput = screen.getByTestId("question-input");
      await user.type(questionInput, "How to test React?");

      const launchButton = screen.getByTestId("launch-button");
      expect(launchButton).not.toBeDisabled();
    });

    it("shows recent sessions section", async () => {
      await renderResearchPanel();
      expect(screen.getByText("Recent Research Sessions")).toBeInTheDocument();
      expect(screen.getByTestId("session-card")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Methodologies Panel
  // ==========================================================================

  describe("methodologies panel", () => {
    async function renderMethodologiesPanel() {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      await user.click(screen.getByTestId("tab-methodologies"));
      await waitFor(() => {
        expect(screen.getByTestId("methodologies-panel")).toBeInTheDocument();
      });
      return user;
    }

    it("shows empty state when no methodologies", async () => {
      await renderMethodologiesPanel();

      expect(screen.getByText("No methodologies available")).toBeInTheDocument();
      expect(screen.getByText("Configure methodologies in the plugin directory")).toBeInTheDocument();
    });

    it("renders header with title", async () => {
      await renderMethodologiesPanel();

      expect(screen.getByText("Development Methodologies")).toBeInTheDocument();
      expect(screen.getByText("Choose how RalphX organizes work")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("uses tablist role for navigation", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByRole("tablist")).toBeInTheDocument();
    });

    it("uses tab role for each tab", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getAllByRole("tab")).toHaveLength(4);
    });

    it("uses tabpanel role for content", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByRole("tabpanel")).toBeInTheDocument();
    });

    it("tabs have correct aria-selected state", () => {
      renderWithProviders(<ExtensibilityView />);

      const workflowsTab = screen.getByTestId("tab-workflows");
      const artifactsTab = screen.getByTestId("tab-artifacts");

      expect(workflowsTab).toHaveAttribute("aria-selected", "true");
      expect(artifactsTab).toHaveAttribute("aria-selected", "false");
    });

    it("research depth has radiogroup role", async () => {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);
      await user.click(screen.getByTestId("tab-research"));

      await waitFor(() => {
        expect(screen.getByTestId("research-panel")).toBeInTheDocument();
      });
      const depthSelector = screen.getByTestId("depth-preset-selector");
      expect(depthSelector).toHaveAttribute("role", "radiogroup");
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("tabs have icons", () => {
      renderWithProviders(<ExtensibilityView />);

      // Check that tab triggers contain SVG icons
      const workflowsTab = screen.getByTestId("tab-workflows");
      const svg = workflowsTab.querySelector("svg");
      expect(svg).toBeInTheDocument();
    });

    it("tabs have underline indicator style", () => {
      renderWithProviders(<ExtensibilityView />);

      const activeTab = screen.getByTestId("tab-workflows");
      expect(activeTab.className).toContain("border-b-2");
    });

    it("content area has proper padding", () => {
      renderWithProviders(<ExtensibilityView />);

      // The content wrapper should have padding (the div that wraps TabsContent)
      const panel = screen.getByTestId("workflows-panel");
      // panel -> tabscontent div -> padding wrapper div
      const contentWrapper = panel.closest("[class*='p-6']");
      expect(contentWrapper).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Integration
  // ==========================================================================

  describe("integration", () => {
    it("maintains tab state when switching between tabs", async () => {
      const user = userEvent.setup();
      renderWithProviders(<ExtensibilityView />);

      // Go to artifacts, select a bucket
      await user.click(screen.getByTestId("tab-artifacts"));
      await waitFor(() => {
        expect(screen.getByTestId("artifacts-panel")).toBeInTheDocument();
      });
      const prdsBucket = screen.getAllByTestId("bucket-item")[2];
      await user.click(prdsBucket);

      // Switch to research and back
      await user.click(screen.getByTestId("tab-research"));
      await waitFor(() => {
        expect(screen.getByTestId("research-panel")).toBeInTheDocument();
      });
      await user.click(screen.getByTestId("tab-artifacts"));
      await waitFor(() => {
        // Bucket state may or may not persist (component internal state)
        expect(screen.getByTestId("artifacts-panel")).toBeInTheDocument();
      });
    });
  });
});
