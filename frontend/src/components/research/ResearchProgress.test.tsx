/**
 * ResearchProgress component tests
 *
 * Tests for:
 * - Displaying research process name and status
 * - Progress bar (currentIteration / maxIterations)
 * - Pause/Resume/Stop buttons
 * - Loading states
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ResearchProgress } from "./ResearchProgress";
import type { ResearchProcess } from "@/types/research";

// ============================================================================
// Test Data
// ============================================================================

const createMockProcess = (overrides: Partial<ResearchProcess> = {}): ResearchProcess => ({
  id: "process-1",
  name: "Architecture Research",
  brief: {
    question: "What is the best approach?",
    constraints: [],
  },
  depth: { type: "preset", preset: "standard" },
  agentProfileId: "deep-researcher",
  output: {
    targetBucket: "research-outputs",
    artifactTypes: [],
  },
  progress: {
    currentIteration: 25,
    status: "running",
  },
  createdAt: "2026-01-24T10:00:00Z",
  startedAt: "2026-01-24T10:05:00Z",
  ...overrides,
});

describe("ResearchProgress", () => {
  const defaultProps = {
    process: createMockProcess(),
    onPause: vi.fn(),
    onResume: vi.fn(),
    onStop: vi.fn(),
    isActionPending: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("research-progress")).toBeInTheDocument();
    });

    it("displays process name", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("process-name")).toHaveTextContent("Architecture Research");
    });

    it("displays status badge", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("status-badge")).toHaveTextContent(/running/i);
    });

    it("displays iteration count", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("iteration-count")).toHaveTextContent("25 / 50");
    });

    it("renders progress bar", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("progress-bar")).toBeInTheDocument();
    });

    it("shows correct progress percentage", () => {
      render(<ResearchProgress {...defaultProps} />);
      const progressFill = screen.getByTestId("progress-fill");
      expect(progressFill).toHaveStyle({ width: "50%" });
    });
  });

  // ==========================================================================
  // Status Variants
  // ==========================================================================

  describe("status variants", () => {
    it("shows running status correctly", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("status-badge")).toHaveTextContent("Running");
    });

    it("shows pending status correctly", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({ progress: { currentIteration: 0, status: "pending" } })}
        />
      );
      expect(screen.getByTestId("status-badge")).toHaveTextContent("Pending");
    });

    it("shows paused status correctly", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({ progress: { currentIteration: 30, status: "paused" } })}
        />
      );
      expect(screen.getByTestId("status-badge")).toHaveTextContent("Paused");
    });

    it("shows completed status correctly", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({ progress: { currentIteration: 50, status: "completed" } })}
        />
      );
      expect(screen.getByTestId("status-badge")).toHaveTextContent("Completed");
    });

    it("shows failed status correctly", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({
            progress: { currentIteration: 20, status: "failed", errorMessage: "Network error" },
          })}
        />
      );
      expect(screen.getByTestId("status-badge")).toHaveTextContent("Failed");
    });
  });

  // ==========================================================================
  // Control Buttons
  // ==========================================================================

  describe("control buttons", () => {
    it("shows pause button when running", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("pause-button")).toBeInTheDocument();
      expect(screen.queryByTestId("resume-button")).not.toBeInTheDocument();
    });

    it("shows resume button when paused", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({ progress: { currentIteration: 30, status: "paused" } })}
        />
      );
      expect(screen.getByTestId("resume-button")).toBeInTheDocument();
      expect(screen.queryByTestId("pause-button")).not.toBeInTheDocument();
    });

    it("shows stop button when running", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByTestId("stop-button")).toBeInTheDocument();
    });

    it("hides control buttons when completed", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({ progress: { currentIteration: 50, status: "completed" } })}
        />
      );
      expect(screen.queryByTestId("pause-button")).not.toBeInTheDocument();
      expect(screen.queryByTestId("stop-button")).not.toBeInTheDocument();
    });

    it("hides control buttons when failed", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({ progress: { currentIteration: 20, status: "failed" } })}
        />
      );
      expect(screen.queryByTestId("pause-button")).not.toBeInTheDocument();
      expect(screen.queryByTestId("stop-button")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Button Actions
  // ==========================================================================

  describe("button actions", () => {
    it("calls onPause when pause button is clicked", () => {
      render(<ResearchProgress {...defaultProps} />);
      fireEvent.click(screen.getByTestId("pause-button"));
      expect(defaultProps.onPause).toHaveBeenCalledWith("process-1");
    });

    it("calls onResume when resume button is clicked", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({ progress: { currentIteration: 30, status: "paused" } })}
        />
      );
      fireEvent.click(screen.getByTestId("resume-button"));
      expect(defaultProps.onResume).toHaveBeenCalledWith("process-1");
    });

    it("calls onStop when stop button is clicked", () => {
      render(<ResearchProgress {...defaultProps} />);
      fireEvent.click(screen.getByTestId("stop-button"));
      expect(defaultProps.onStop).toHaveBeenCalledWith("process-1");
    });
  });

  // ==========================================================================
  // Loading State
  // ==========================================================================

  describe("loading state", () => {
    it("disables buttons when action is pending", () => {
      render(<ResearchProgress {...defaultProps} isActionPending />);
      expect(screen.getByTestId("pause-button")).toBeDisabled();
      expect(screen.getByTestId("stop-button")).toBeDisabled();
    });
  });

  // ==========================================================================
  // Custom Depth Progress
  // ==========================================================================

  describe("custom depth progress", () => {
    it("calculates progress correctly for custom depth", () => {
      render(
        <ResearchProgress
          {...defaultProps}
          process={createMockProcess({
            depth: { type: "custom", config: { maxIterations: 200, timeoutHours: 8, checkpointInterval: 25 } },
            progress: { currentIteration: 100, status: "running" },
          })}
        />
      );
      const progressFill = screen.getByTestId("progress-fill");
      expect(progressFill).toHaveStyle({ width: "50%" });
      expect(screen.getByTestId("iteration-count")).toHaveTextContent("100 / 200");
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("progress bar has proper role", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByRole("progressbar")).toBeInTheDocument();
    });

    it("progress bar has aria-valuenow", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "50");
    });

    it("buttons have accessible labels", () => {
      render(<ResearchProgress {...defaultProps} />);
      expect(screen.getByRole("button", { name: /pause/i })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /stop/i })).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<ResearchProgress {...defaultProps} />);
      const progress = screen.getByTestId("research-progress");
      expect(progress).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses status color for running status", () => {
      render(<ResearchProgress {...defaultProps} />);
      const badge = screen.getByTestId("status-badge");
      expect(badge).toHaveStyle({ color: "var(--status-info)" });
    });

    it("uses accent color for progress bar", () => {
      render(<ResearchProgress {...defaultProps} />);
      const fill = screen.getByTestId("progress-fill");
      expect(fill).toHaveStyle({ backgroundColor: "var(--accent-primary)" });
    });
  });
});
