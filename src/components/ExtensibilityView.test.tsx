/**
 * ExtensibilityView component tests
 *
 * Tests for:
 * - Tab navigation (Workflows, Artifacts, Research, Methodologies)
 * - Tab content rendering
 * - Default tab selection
 * - Tab switching
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createElement } from "react";
import { ExtensibilityView } from "./ExtensibilityView";

// Mock child components
vi.mock("@/components/workflows/WorkflowEditor", () => ({
  WorkflowEditor: () => <div data-testid="workflow-editor">WorkflowEditor</div>,
}));

vi.mock("@/components/artifacts/ArtifactBrowser", () => ({
  ArtifactBrowser: () => <div data-testid="artifact-browser">ArtifactBrowser</div>,
}));

vi.mock("@/components/research/ResearchLauncher", () => ({
  ResearchLauncher: () => <div data-testid="research-launcher">ResearchLauncher</div>,
}));

vi.mock("@/components/methodologies/MethodologyBrowser", () => ({
  MethodologyBrowser: () => <div data-testid="methodology-browser">MethodologyBrowser</div>,
}));

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
    createElement(QueryClientProvider, { client: queryClient }, ui)
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
  });

  // ==========================================================================
  // Default Tab
  // ==========================================================================

  describe("default tab", () => {
    it("shows Workflows tab as active by default", () => {
      renderWithProviders(<ExtensibilityView />);
      const workflowsTab = screen.getByTestId("tab-workflows");
      expect(workflowsTab).toHaveAttribute("aria-selected", "true");
    });

    it("renders Workflows content by default", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("workflow-editor")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Tab Switching
  // ==========================================================================

  describe("tab switching", () => {
    it("switches to Artifacts tab when clicked", () => {
      renderWithProviders(<ExtensibilityView />);
      fireEvent.click(screen.getByTestId("tab-artifacts"));

      expect(screen.getByTestId("tab-artifacts")).toHaveAttribute("aria-selected", "true");
      expect(screen.getByTestId("tab-workflows")).toHaveAttribute("aria-selected", "false");
      expect(screen.getByTestId("artifact-browser")).toBeInTheDocument();
    });

    it("switches to Research tab when clicked", () => {
      renderWithProviders(<ExtensibilityView />);
      fireEvent.click(screen.getByTestId("tab-research"));

      expect(screen.getByTestId("tab-research")).toHaveAttribute("aria-selected", "true");
      expect(screen.getByTestId("research-launcher")).toBeInTheDocument();
    });

    it("switches to Methodologies tab when clicked", () => {
      renderWithProviders(<ExtensibilityView />);
      fireEvent.click(screen.getByTestId("tab-methodologies"));

      expect(screen.getByTestId("tab-methodologies")).toHaveAttribute("aria-selected", "true");
      expect(screen.getByTestId("methodology-browser")).toBeInTheDocument();
    });

    it("hides previous tab content when switching", () => {
      renderWithProviders(<ExtensibilityView />);
      expect(screen.getByTestId("workflow-editor")).toBeInTheDocument();

      fireEvent.click(screen.getByTestId("tab-artifacts"));

      expect(screen.queryByTestId("workflow-editor")).not.toBeInTheDocument();
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

    it("links tab to tabpanel via aria-controls", () => {
      renderWithProviders(<ExtensibilityView />);
      const workflowsTab = screen.getByTestId("tab-workflows");
      const panel = screen.getByRole("tabpanel");
      expect(workflowsTab.getAttribute("aria-controls")).toBe(panel.id);
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      renderWithProviders(<ExtensibilityView />);
      const view = screen.getByTestId("extensibility-view");
      expect(view).toHaveStyle({ backgroundColor: "var(--bg-base)" });
    });

    it("uses accent color for active tab", () => {
      renderWithProviders(<ExtensibilityView />);
      const activeTab = screen.getByTestId("tab-workflows");
      const style = activeTab.getAttribute("style");
      expect(style).toContain("border-color: var(--accent-primary)");
    });

    it("uses muted color for inactive tabs", () => {
      renderWithProviders(<ExtensibilityView />);
      const inactiveTab = screen.getByTestId("tab-artifacts");
      const style = inactiveTab.getAttribute("style");
      expect(style).toContain("border-color: transparent");
    });
  });
});
