/**
 * ResearchResults component tests
 *
 * Tests for:
 * - Displaying artifacts produced by research
 * - Links to artifact browser
 * - Summary of findings/recommendations
 * - Empty state
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { ResearchResults } from "./ResearchResults";
import type { Artifact } from "@/types/artifact";
import type { ResearchProcess } from "@/types/research";

// ============================================================================
// Test Data
// ============================================================================

const createMockArtifact = (overrides: Partial<Artifact> = {}): Artifact => ({
  id: "artifact-1",
  type: "research_document",
  name: "Research Summary",
  content: { type: "inline", text: "Summary of findings..." },
  metadata: {
    createdAt: "2026-01-24T12:00:00Z",
    createdBy: "deep-researcher",
    version: 1,
  },
  derivedFrom: [],
  bucketId: "research-outputs",
  ...overrides,
});

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
    artifactTypes: ["research_document", "findings", "recommendations"],
  },
  progress: {
    currentIteration: 50,
    status: "completed",
  },
  createdAt: "2026-01-24T10:00:00Z",
  startedAt: "2026-01-24T10:05:00Z",
  completedAt: "2026-01-24T12:00:00Z",
  ...overrides,
});

const mockArtifacts: Artifact[] = [
  createMockArtifact({ id: "a1", name: "Research Summary", type: "research_document" }),
  createMockArtifact({ id: "a2", name: "Key Findings", type: "findings" }),
  createMockArtifact({ id: "a3", name: "Recommendations", type: "recommendations" }),
];

describe("ResearchResults", () => {
  const defaultProps = {
    process: createMockProcess(),
    artifacts: mockArtifacts,
    onViewArtifact: vi.fn(),
    onViewInBrowser: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<ResearchResults {...defaultProps} />);
      expect(screen.getByTestId("research-results")).toBeInTheDocument();
    });

    it("displays process name", () => {
      render(<ResearchResults {...defaultProps} />);
      expect(screen.getByTestId("process-name")).toHaveTextContent("Architecture Research");
    });

    it("displays completed status", () => {
      render(<ResearchResults {...defaultProps} />);
      expect(screen.getByTestId("status-badge")).toHaveTextContent("Completed");
    });

    it("displays artifact count", () => {
      render(<ResearchResults {...defaultProps} />);
      expect(screen.getByTestId("artifact-count")).toHaveTextContent("3 artifacts");
    });

    it("renders all artifacts", () => {
      render(<ResearchResults {...defaultProps} />);
      const items = screen.getAllByTestId("artifact-item");
      expect(items).toHaveLength(3);
    });
  });

  // ==========================================================================
  // Artifact Display
  // ==========================================================================

  describe("artifact display", () => {
    it("shows artifact names in items", () => {
      render(<ResearchResults {...defaultProps} />);
      const items = screen.getAllByTestId("artifact-item");
      expect(items[0]).toHaveTextContent("Research Summary");
      expect(items[1]).toHaveTextContent("Key Findings");
      expect(items[2]).toHaveTextContent("Recommendations");
    });

    it("shows artifact type badge", () => {
      render(<ResearchResults {...defaultProps} />);
      const items = screen.getAllByTestId("artifact-item");
      expect(within(items[0]).getByTestId("artifact-type")).toHaveTextContent("Research Document");
      expect(within(items[1]).getByTestId("artifact-type")).toHaveTextContent("Findings");
      expect(within(items[2]).getByTestId("artifact-type")).toHaveTextContent("Recommendations");
    });
  });

  // ==========================================================================
  // Artifact Actions
  // ==========================================================================

  describe("artifact actions", () => {
    it("calls onViewArtifact when artifact is clicked", () => {
      render(<ResearchResults {...defaultProps} />);
      const items = screen.getAllByTestId("artifact-item");
      fireEvent.click(items[0]);
      expect(defaultProps.onViewArtifact).toHaveBeenCalledWith("a1");
    });

    it("calls onViewInBrowser when view in browser button clicked", () => {
      render(<ResearchResults {...defaultProps} />);
      fireEvent.click(screen.getByTestId("view-in-browser-button"));
      expect(defaultProps.onViewInBrowser).toHaveBeenCalledWith("research-outputs");
    });
  });

  // ==========================================================================
  // Research Question
  // ==========================================================================

  describe("research question", () => {
    it("displays the research question", () => {
      render(<ResearchResults {...defaultProps} />);
      expect(screen.getByTestId("research-question")).toHaveTextContent("What is the best approach?");
    });
  });

  // ==========================================================================
  // Empty State
  // ==========================================================================

  describe("empty state", () => {
    it("shows empty message when no artifacts", () => {
      render(<ResearchResults {...defaultProps} artifacts={[]} />);
      expect(screen.getByText(/no artifacts/i)).toBeInTheDocument();
    });

    it("hides artifact count when no artifacts", () => {
      render(<ResearchResults {...defaultProps} artifacts={[]} />);
      expect(screen.queryByTestId("artifact-count")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Failed State
  // ==========================================================================

  describe("failed state", () => {
    it("shows failed status for failed process", () => {
      render(
        <ResearchResults
          {...defaultProps}
          process={createMockProcess({
            progress: { currentIteration: 20, status: "failed", errorMessage: "Rate limit exceeded" },
          })}
        />
      );
      expect(screen.getByTestId("status-badge")).toHaveTextContent("Failed");
    });

    it("shows error message when process failed", () => {
      render(
        <ResearchResults
          {...defaultProps}
          process={createMockProcess({
            progress: { currentIteration: 20, status: "failed", errorMessage: "Rate limit exceeded" },
          })}
        />
      );
      expect(screen.getByTestId("error-message")).toHaveTextContent("Rate limit exceeded");
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("artifact items have button role", () => {
      render(<ResearchResults {...defaultProps} />);
      const items = screen.getAllByTestId("artifact-item");
      items.forEach((item) => {
        expect(item.tagName).toBe("BUTTON");
      });
    });

    it("artifact items have accessible names", () => {
      render(<ResearchResults {...defaultProps} />);
      expect(screen.getByRole("button", { name: /research summary/i })).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<ResearchResults {...defaultProps} />);
      const results = screen.getByTestId("research-results");
      expect(results).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });

    it("uses success color for completed status", () => {
      render(<ResearchResults {...defaultProps} />);
      const badge = screen.getByTestId("status-badge");
      expect(badge).toHaveStyle({ color: "var(--status-success)" });
    });

    it("uses error color for failed status", () => {
      render(
        <ResearchResults
          {...defaultProps}
          process={createMockProcess({
            progress: { currentIteration: 20, status: "failed" },
          })}
        />
      );
      const badge = screen.getByTestId("status-badge");
      expect(badge).toHaveStyle({ color: "var(--status-error)" });
    });
  });
});
