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

describe("ExtensibilityView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<ExtensibilityView />);
      expect(screen.getByTestId("extensibility-view")).toBeInTheDocument();
    });

    it("renders tab navigation", () => {
      render(<ExtensibilityView />);
      expect(screen.getByTestId("tab-navigation")).toBeInTheDocument();
    });

    it("renders all four tabs", () => {
      render(<ExtensibilityView />);
      expect(screen.getByTestId("tab-workflows")).toBeInTheDocument();
      expect(screen.getByTestId("tab-artifacts")).toBeInTheDocument();
      expect(screen.getByTestId("tab-research")).toBeInTheDocument();
      expect(screen.getByTestId("tab-methodologies")).toBeInTheDocument();
    });

    it("displays tab labels", () => {
      render(<ExtensibilityView />);
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
      render(<ExtensibilityView />);
      const workflowsTab = screen.getByTestId("tab-workflows");
      expect(workflowsTab).toHaveAttribute("aria-selected", "true");
    });

    it("renders Workflows content by default", () => {
      render(<ExtensibilityView />);
      expect(screen.getByTestId("workflow-editor")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Tab Switching
  // ==========================================================================

  describe("tab switching", () => {
    it("switches to Artifacts tab when clicked", () => {
      render(<ExtensibilityView />);
      fireEvent.click(screen.getByTestId("tab-artifacts"));

      expect(screen.getByTestId("tab-artifacts")).toHaveAttribute("aria-selected", "true");
      expect(screen.getByTestId("tab-workflows")).toHaveAttribute("aria-selected", "false");
      expect(screen.getByTestId("artifact-browser")).toBeInTheDocument();
    });

    it("switches to Research tab when clicked", () => {
      render(<ExtensibilityView />);
      fireEvent.click(screen.getByTestId("tab-research"));

      expect(screen.getByTestId("tab-research")).toHaveAttribute("aria-selected", "true");
      expect(screen.getByTestId("research-launcher")).toBeInTheDocument();
    });

    it("switches to Methodologies tab when clicked", () => {
      render(<ExtensibilityView />);
      fireEvent.click(screen.getByTestId("tab-methodologies"));

      expect(screen.getByTestId("tab-methodologies")).toHaveAttribute("aria-selected", "true");
      expect(screen.getByTestId("methodology-browser")).toBeInTheDocument();
    });

    it("hides previous tab content when switching", () => {
      render(<ExtensibilityView />);
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
      render(<ExtensibilityView />);
      expect(screen.getByRole("tablist")).toBeInTheDocument();
    });

    it("uses tab role for each tab", () => {
      render(<ExtensibilityView />);
      expect(screen.getAllByRole("tab")).toHaveLength(4);
    });

    it("uses tabpanel role for content", () => {
      render(<ExtensibilityView />);
      expect(screen.getByRole("tabpanel")).toBeInTheDocument();
    });

    it("links tab to tabpanel via aria-controls", () => {
      render(<ExtensibilityView />);
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
      render(<ExtensibilityView />);
      const view = screen.getByTestId("extensibility-view");
      expect(view).toHaveStyle({ backgroundColor: "var(--bg-base)" });
    });

    it("uses accent color for active tab", () => {
      render(<ExtensibilityView />);
      const activeTab = screen.getByTestId("tab-workflows");
      const style = activeTab.getAttribute("style");
      expect(style).toContain("border-color: var(--accent-primary)");
    });

    it("uses muted color for inactive tabs", () => {
      render(<ExtensibilityView />);
      const inactiveTab = screen.getByTestId("tab-artifacts");
      const style = inactiveTab.getAttribute("style");
      expect(style).toContain("border-color: transparent");
    });
  });
});
