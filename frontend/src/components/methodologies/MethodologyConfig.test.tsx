/**
 * MethodologyConfig component tests
 *
 * Tests for:
 * - Displaying active methodology details
 * - Workflow columns with color chips
 * - Phase progression diagram
 * - Agent profiles list
 * - Empty state (no active methodology)
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import { MethodologyConfig } from "./MethodologyConfig";
import type { MethodologyExtension } from "@/types/methodology";

// ============================================================================
// Test Data
// ============================================================================

const createMockMethodology = (overrides: Partial<MethodologyExtension> = {}): MethodologyExtension => ({
  id: "methodology-1",
  name: "BMAD",
  description: "Business Model Aligned Development",
  agentProfiles: ["analyst", "architect", "pm"],
  skills: [],
  workflow: {
    id: "bmad-workflow",
    name: "BMAD Workflow",
    columns: [
      { name: "Backlog", mapsTo: "pending" },
      { name: "In Progress", mapsTo: "in_progress" },
      { name: "Review", mapsTo: "code_review" },
      { name: "Done", mapsTo: "completed" },
    ],
    isDefault: false,
  },
  phases: [
    { id: "p1", name: "Discovery", order: 1 },
    { id: "p2", name: "Design", order: 2 },
    { id: "p3", name: "Build", order: 3 },
  ],
  templates: [],
  isActive: true,
  createdAt: "2026-01-24T10:00:00Z",
  ...overrides,
});

describe("MethodologyConfig", () => {
  const defaultProps = {
    methodology: createMockMethodology(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("methodology-config")).toBeInTheDocument();
    });

    it("displays methodology name", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("methodology-name")).toHaveTextContent("BMAD");
    });

    it("displays methodology description", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("methodology-description")).toHaveTextContent("Business Model Aligned Development");
    });
  });

  // ==========================================================================
  // Workflow Columns
  // ==========================================================================

  describe("workflow columns", () => {
    it("displays workflow section", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("workflow-section")).toBeInTheDocument();
    });

    it("displays all workflow columns", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const columns = screen.getAllByTestId("workflow-column");
      expect(columns).toHaveLength(4);
    });

    it("shows column names", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByText("Backlog")).toBeInTheDocument();
      expect(screen.getByText("In Progress")).toBeInTheDocument();
      expect(screen.getByText("Review")).toBeInTheDocument();
      expect(screen.getByText("Done")).toBeInTheDocument();
    });

    it("shows color chips for columns", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const chips = screen.getAllByTestId("column-chip");
      expect(chips).toHaveLength(4);
    });

    it("shows mapped status for columns", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const columns = screen.getAllByTestId("workflow-column");
      expect(within(columns[0]).getByTestId("mapped-status")).toHaveTextContent("pending");
      expect(within(columns[2]).getByTestId("mapped-status")).toHaveTextContent("code_review");
    });
  });

  // ==========================================================================
  // Phase Progression
  // ==========================================================================

  describe("phase progression", () => {
    it("displays phases section", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("phases-section")).toBeInTheDocument();
    });

    it("displays all phases", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const phases = screen.getAllByTestId("phase-item");
      expect(phases).toHaveLength(3);
    });

    it("shows phase names in order", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const phases = screen.getAllByTestId("phase-item");
      expect(within(phases[0]).getByTestId("phase-name")).toHaveTextContent("Discovery");
      expect(within(phases[1]).getByTestId("phase-name")).toHaveTextContent("Design");
      expect(within(phases[2]).getByTestId("phase-name")).toHaveTextContent("Build");
    });

    it("shows phase order numbers", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const phases = screen.getAllByTestId("phase-item");
      expect(within(phases[0]).getByTestId("phase-order")).toHaveTextContent("1");
      expect(within(phases[1]).getByTestId("phase-order")).toHaveTextContent("2");
      expect(within(phases[2]).getByTestId("phase-order")).toHaveTextContent("3");
    });

    it("shows arrows between phases", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const arrows = screen.getAllByTestId("phase-arrow");
      expect(arrows).toHaveLength(2);
    });
  });

  // ==========================================================================
  // Agent Profiles
  // ==========================================================================

  describe("agent profiles", () => {
    it("displays agents section", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("agents-section")).toBeInTheDocument();
    });

    it("displays all agent profiles", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const agents = screen.getAllByTestId("agent-item");
      expect(agents).toHaveLength(3);
    });

    it("shows agent profile IDs", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByText("analyst")).toBeInTheDocument();
      expect(screen.getByText("architect")).toBeInTheDocument();
      expect(screen.getByText("pm")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Empty State
  // ==========================================================================

  describe("empty state", () => {
    it("shows empty message when no methodology", () => {
      render(<MethodologyConfig methodology={null} />);
      expect(screen.getByText(/no active methodology/i)).toBeInTheDocument();
    });

    it("hides configuration when no methodology", () => {
      render(<MethodologyConfig methodology={null} />);
      expect(screen.queryByTestId("methodology-name")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("uses list for phases", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("phases-section").querySelector("ul")).toBeInTheDocument();
    });

    it("uses list for agents", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByTestId("agents-section").querySelector("ul")).toBeInTheDocument();
    });

    it("uses section headings", () => {
      render(<MethodologyConfig {...defaultProps} />);
      expect(screen.getByText("Workflow")).toBeInTheDocument();
      expect(screen.getByText("Phases")).toBeInTheDocument();
      expect(screen.getByText("Agents")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const config = screen.getByTestId("methodology-config");
      expect(config).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses muted color for section headers", () => {
      render(<MethodologyConfig {...defaultProps} />);
      const workflowHeader = screen.getByText("Workflow");
      expect(workflowHeader).toHaveStyle({ color: "var(--text-muted)" });
    });
  });
});
