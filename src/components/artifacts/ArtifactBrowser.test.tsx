/**
 * ArtifactBrowser component tests
 *
 * Tests for:
 * - Bucket sidebar rendering
 * - Artifact list display
 * - Bucket selection filtering
 * - Type filter (optional)
 * - Empty states
 * - Loading state
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { ArtifactBrowser } from "./ArtifactBrowser";
import type { Artifact, ArtifactBucket } from "@/types/artifact";

// ============================================================================
// Test Data
// ============================================================================

const createMockArtifact = (overrides: Partial<Artifact> = {}): Artifact => ({
  id: "artifact-1",
  type: "prd",
  name: "Test PRD",
  content: { type: "inline", text: "Test content" },
  metadata: {
    createdAt: "2026-01-24T10:00:00Z",
    createdBy: "user",
    version: 1,
  },
  derivedFrom: [],
  bucketId: "prd-library",
  ...overrides,
});

const createMockBucket = (overrides: Partial<ArtifactBucket> = {}): ArtifactBucket => ({
  id: "prd-library",
  name: "PRD Library",
  acceptedTypes: ["prd", "specification", "design_doc"],
  writers: ["orchestrator", "user"],
  readers: ["all"],
  isSystem: true,
  ...overrides,
});

const mockBuckets: ArtifactBucket[] = [
  createMockBucket({ id: "prd-library", name: "PRD Library" }),
  createMockBucket({ id: "research-outputs", name: "Research Outputs", acceptedTypes: ["research_document", "findings"] }),
  createMockBucket({ id: "code-changes", name: "Code Changes", acceptedTypes: ["code_change", "diff"] }),
];

const mockArtifacts: Artifact[] = [
  createMockArtifact({ id: "a1", name: "Main PRD", bucketId: "prd-library", type: "prd" }),
  createMockArtifact({ id: "a2", name: "Design Doc", bucketId: "prd-library", type: "design_doc" }),
  createMockArtifact({ id: "a3", name: "Research 1", bucketId: "research-outputs", type: "research_document" }),
  createMockArtifact({ id: "a4", name: "Findings", bucketId: "research-outputs", type: "findings" }),
  createMockArtifact({ id: "a5", name: "Code Diff", bucketId: "code-changes", type: "diff" }),
];

describe("ArtifactBrowser", () => {
  const defaultProps = {
    buckets: mockBuckets,
    artifacts: mockArtifacts,
    selectedBucketId: "prd-library",
    selectedArtifactId: null as string | null,
    onSelectBucket: vi.fn(),
    onSelectArtifact: vi.fn(),
    isLoading: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      expect(screen.getByTestId("artifact-browser")).toBeInTheDocument();
    });

    it("renders bucket sidebar", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      expect(screen.getByTestId("bucket-sidebar")).toBeInTheDocument();
    });

    it("renders artifact list area", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      expect(screen.getByTestId("artifact-list")).toBeInTheDocument();
    });

    it("displays all buckets in sidebar", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const sidebar = screen.getByTestId("bucket-sidebar");
      expect(within(sidebar).getByText("PRD Library")).toBeInTheDocument();
      expect(within(sidebar).getByText("Research Outputs")).toBeInTheDocument();
      expect(within(sidebar).getByText("Code Changes")).toBeInTheDocument();
    });

    it("shows bucket item count", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const bucketItems = screen.getAllByTestId("bucket-item");
      expect(within(bucketItems[0]).getByTestId("bucket-count")).toHaveTextContent("2");
      expect(within(bucketItems[1]).getByTestId("bucket-count")).toHaveTextContent("2");
      expect(within(bucketItems[2]).getByTestId("bucket-count")).toHaveTextContent("1");
    });
  });

  // ==========================================================================
  // Bucket Selection
  // ==========================================================================

  describe("bucket selection", () => {
    it("highlights selected bucket", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const bucketItems = screen.getAllByTestId("bucket-item");
      expect(bucketItems[0]).toHaveAttribute("data-selected", "true");
      expect(bucketItems[1]).toHaveAttribute("data-selected", "false");
    });

    it("calls onSelectBucket when bucket is clicked", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const bucketItems = screen.getAllByTestId("bucket-item");
      fireEvent.click(bucketItems[1]);
      expect(defaultProps.onSelectBucket).toHaveBeenCalledWith("research-outputs");
    });

    it("filters artifacts by selected bucket", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const artifactList = screen.getByTestId("artifact-list");
      // Should only show PRD Library artifacts (2 items)
      expect(within(artifactList).getAllByTestId("artifact-card")).toHaveLength(2);
    });

    it("shows different artifacts when bucket changes", () => {
      const { rerender } = render(<ArtifactBrowser {...defaultProps} />);

      // Rerender with different bucket selected
      rerender(<ArtifactBrowser {...defaultProps} selectedBucketId="research-outputs" />);

      const artifactList = screen.getByTestId("artifact-list");
      expect(within(artifactList).getAllByTestId("artifact-card")).toHaveLength(2);
      expect(within(artifactList).getByText("Research 1")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Artifact Selection
  // ==========================================================================

  describe("artifact selection", () => {
    it("calls onSelectArtifact when artifact is clicked", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const artifactCards = screen.getAllByTestId("artifact-card");
      fireEvent.click(artifactCards[0]);
      expect(defaultProps.onSelectArtifact).toHaveBeenCalledWith("a1");
    });

    it("highlights selected artifact", () => {
      render(<ArtifactBrowser {...defaultProps} selectedArtifactId="a1" />);
      const artifactCards = screen.getAllByTestId("artifact-card");
      expect(artifactCards[0]).toHaveAttribute("data-selected", "true");
      expect(artifactCards[1]).toHaveAttribute("data-selected", "false");
    });
  });

  // ==========================================================================
  // Empty States
  // ==========================================================================

  describe("empty states", () => {
    it("shows empty message when no buckets", () => {
      render(<ArtifactBrowser {...defaultProps} buckets={[]} />);
      expect(screen.getByText(/no buckets/i)).toBeInTheDocument();
    });

    it("shows empty message when bucket has no artifacts", () => {
      render(<ArtifactBrowser {...defaultProps} artifacts={[]} />);
      expect(screen.getByText(/no artifacts/i)).toBeInTheDocument();
    });

    it("shows empty message when no bucket selected", () => {
      render(<ArtifactBrowser {...defaultProps} selectedBucketId={null} />);
      expect(screen.getByText(/select a bucket/i)).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Loading State
  // ==========================================================================

  describe("loading state", () => {
    it("shows loading indicator when isLoading", () => {
      render(<ArtifactBrowser {...defaultProps} isLoading />);
      expect(screen.getByTestId("loading-indicator")).toBeInTheDocument();
    });

    it("disables bucket selection when loading", () => {
      render(<ArtifactBrowser {...defaultProps} isLoading />);
      const bucketItems = screen.getAllByTestId("bucket-item");
      fireEvent.click(bucketItems[1]);
      expect(defaultProps.onSelectBucket).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // System Bucket Indicator
  // ==========================================================================

  describe("system bucket indicator", () => {
    it("shows system badge for system buckets", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const bucketItems = screen.getAllByTestId("bucket-item");
      expect(within(bucketItems[0]).getByTestId("system-badge")).toBeInTheDocument();
    });

    it("does not show system badge for non-system buckets", () => {
      const customBuckets = [
        createMockBucket({ id: "custom", name: "Custom", isSystem: false }),
      ];
      render(<ArtifactBrowser {...defaultProps} buckets={customBuckets} selectedBucketId="custom" />);
      const bucketItems = screen.getAllByTestId("bucket-item");
      expect(within(bucketItems[0]).queryByTestId("system-badge")).not.toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("has navigation role for bucket sidebar", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      expect(screen.getByRole("navigation")).toBeInTheDocument();
    });

    it("bucket items have button role", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const sidebar = screen.getByTestId("bucket-sidebar");
      expect(within(sidebar).getAllByRole("button")).toHaveLength(3);
    });

    it("bucket buttons have accessible names", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      expect(screen.getByRole("button", { name: /prd library/i })).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const browser = screen.getByTestId("artifact-browser");
      expect(browser).toHaveStyle({ backgroundColor: "var(--bg-base)" });
    });

    it("uses design tokens for sidebar background", () => {
      render(<ArtifactBrowser {...defaultProps} />);
      const sidebar = screen.getByTestId("bucket-sidebar");
      expect(sidebar).toHaveStyle({ backgroundColor: "var(--bg-surface)" });
    });
  });
});
