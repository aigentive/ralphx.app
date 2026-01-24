/**
 * ArtifactFlow component tests
 *
 * Tests for:
 * - Rendering flow name and trigger
 * - Rendering flow steps
 * - Active/inactive state
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import { ArtifactFlow } from "./ArtifactFlow";
import type { ArtifactFlow as ArtifactFlowType } from "@/types/artifact";

// ============================================================================
// Test Data
// ============================================================================

const createMockFlow = (overrides: Partial<ArtifactFlowType> = {}): ArtifactFlowType => ({
  id: "flow-1",
  name: "Research to Dev",
  trigger: {
    event: "artifact_created",
    filter: {
      artifactTypes: ["recommendations"],
      sourceBucket: "research-outputs",
    },
  },
  steps: [
    { type: "copy", toBucket: "prd-library" },
    { type: "spawn_process", processType: "task_decomposition", agentProfile: "orchestrator" },
  ],
  isActive: true,
  createdAt: "2026-01-24T10:00:00Z",
  ...overrides,
});

describe("ArtifactFlow", () => {
  const defaultProps = {
    flow: createMockFlow(),
  };

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getByTestId("artifact-flow")).toBeInTheDocument();
    });

    it("displays flow name", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getByTestId("flow-name")).toHaveTextContent("Research to Dev");
    });

    it("displays trigger event", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getByTestId("flow-trigger")).toHaveTextContent("artifact_created");
    });

    it("displays trigger filter when present", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const trigger = screen.getByTestId("flow-trigger");
      expect(within(trigger).getByText(/recommendations/i)).toBeInTheDocument();
      expect(within(trigger).getByText(/research-outputs/i)).toBeInTheDocument();
    });

    it("renders all flow steps", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const steps = screen.getAllByTestId("flow-step");
      expect(steps).toHaveLength(2);
    });

    it("displays copy step correctly", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const steps = screen.getAllByTestId("flow-step");
      expect(within(steps[0]).getByText(/copy/i)).toBeInTheDocument();
      expect(within(steps[0]).getByText(/prd-library/i)).toBeInTheDocument();
    });

    it("displays spawn_process step correctly", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const steps = screen.getAllByTestId("flow-step");
      expect(within(steps[1]).getByText(/spawn/i)).toBeInTheDocument();
      expect(within(steps[1]).getByText(/task_decomposition/i)).toBeInTheDocument();
      expect(within(steps[1]).getByText(/orchestrator/i)).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Trigger Without Filter
  // ==========================================================================

  describe("trigger without filter", () => {
    it("shows trigger event without filter details", () => {
      const flowWithoutFilter = createMockFlow({
        trigger: { event: "task_completed" },
      });
      render(<ArtifactFlow flow={flowWithoutFilter} />);
      expect(screen.getByTestId("flow-trigger")).toHaveTextContent("task_completed");
      expect(screen.queryByTestId("trigger-filter")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Active/Inactive State
  // ==========================================================================

  describe("active/inactive state", () => {
    it("shows active indicator when flow is active", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getByTestId("flow-status")).toHaveTextContent("Active");
    });

    it("shows inactive indicator when flow is inactive", () => {
      render(<ArtifactFlow flow={createMockFlow({ isActive: false })} />);
      expect(screen.getByTestId("flow-status")).toHaveTextContent("Inactive");
    });

    it("uses different styling for inactive flow", () => {
      render(<ArtifactFlow flow={createMockFlow({ isActive: false })} />);
      const flow = screen.getByTestId("artifact-flow");
      expect(flow).toHaveAttribute("data-active", "false");
    });
  });

  // ==========================================================================
  // Step Connections
  // ==========================================================================

  describe("step connections", () => {
    it("shows arrows between steps", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getAllByTestId("step-arrow")).toHaveLength(1);
    });

    it("shows arrow from trigger to first step", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getByTestId("trigger-arrow")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Step Icons
  // ==========================================================================

  describe("step icons", () => {
    it("shows copy icon for copy steps", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const steps = screen.getAllByTestId("flow-step");
      expect(within(steps[0]).getByTestId("step-icon-copy")).toBeInTheDocument();
    });

    it("shows spawn icon for spawn_process steps", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const steps = screen.getAllByTestId("flow-step");
      expect(within(steps[1]).getByTestId("step-icon-spawn")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("uses article role for flow", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getByRole("article")).toBeInTheDocument();
    });

    it("uses list for steps", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getByRole("list")).toBeInTheDocument();
    });

    it("uses listitem for each step", () => {
      render(<ArtifactFlow {...defaultProps} />);
      expect(screen.getAllByRole("listitem")).toHaveLength(2);
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const flow = screen.getByTestId("artifact-flow");
      expect(flow).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses accent color for active status", () => {
      render(<ArtifactFlow {...defaultProps} />);
      const status = screen.getByTestId("flow-status");
      expect(status).toHaveStyle({ color: "var(--status-success)" });
    });

    it("uses muted color for inactive status", () => {
      render(<ArtifactFlow flow={createMockFlow({ isActive: false })} />);
      const status = screen.getByTestId("flow-status");
      expect(status).toHaveStyle({ color: "var(--text-muted)" });
    });
  });
});
