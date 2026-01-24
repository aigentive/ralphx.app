/**
 * ArtifactCard component tests
 *
 * Tests for:
 * - Rendering artifact info (name, type badge, timestamp)
 * - Version display (only when > 1)
 * - Click handling for selection
 * - Selected state styling
 * - Accessibility
 * - Styling with design tokens
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ArtifactCard } from "./ArtifactCard";
import type { Artifact } from "@/types/artifact";

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
  ...overrides,
});

describe("ArtifactCard", () => {
  const defaultProps = {
    artifact: createMockArtifact(),
    onClick: vi.fn(),
    isSelected: false,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("renders component with testid", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.getByTestId("artifact-card")).toBeInTheDocument();
    });

    it("displays artifact name", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.getByTestId("artifact-name")).toHaveTextContent("Test PRD");
    });

    it("displays artifact type badge", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.getByTestId("artifact-type-badge")).toHaveTextContent("PRD");
    });

    it("displays formatted timestamp", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.getByTestId("artifact-timestamp")).toBeInTheDocument();
    });

    it("displays correct type badge for different types", () => {
      render(
        <ArtifactCard
          {...defaultProps}
          artifact={createMockArtifact({ type: "research_document" })}
        />
      );
      expect(screen.getByTestId("artifact-type-badge")).toHaveTextContent("Research Document");
    });
  });

  // ==========================================================================
  // Version Display
  // ==========================================================================

  describe("version display", () => {
    it("does not show version badge when version is 1", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.queryByTestId("artifact-version")).not.toBeInTheDocument();
    });

    it("shows version badge when version is greater than 1", () => {
      render(
        <ArtifactCard
          {...defaultProps}
          artifact={createMockArtifact({
            metadata: { createdAt: "2026-01-24T10:00:00Z", createdBy: "user", version: 3 },
          })}
        />
      );
      expect(screen.getByTestId("artifact-version")).toHaveTextContent("v3");
    });
  });

  // ==========================================================================
  // Click Handling
  // ==========================================================================

  describe("click handling", () => {
    it("calls onClick with artifact id when clicked", () => {
      render(<ArtifactCard {...defaultProps} />);
      fireEvent.click(screen.getByTestId("artifact-card"));
      expect(defaultProps.onClick).toHaveBeenCalledWith("artifact-1");
    });

    it("does not call onClick when disabled", () => {
      render(<ArtifactCard {...defaultProps} disabled />);
      fireEvent.click(screen.getByTestId("artifact-card"));
      expect(defaultProps.onClick).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Selected State
  // ==========================================================================

  describe("selected state", () => {
    it("applies selected styling when isSelected is true", () => {
      render(<ArtifactCard {...defaultProps} isSelected />);
      const card = screen.getByTestId("artifact-card");
      expect(card).toHaveAttribute("data-selected", "true");
    });

    it("does not apply selected styling when isSelected is false", () => {
      render(<ArtifactCard {...defaultProps} />);
      const card = screen.getByTestId("artifact-card");
      expect(card).toHaveAttribute("data-selected", "false");
    });

    it("uses accent border color when selected", () => {
      render(<ArtifactCard {...defaultProps} isSelected />);
      const card = screen.getByTestId("artifact-card");
      expect(card.getAttribute("style")).toContain("border-color: var(--accent-primary)");
    });
  });

  // ==========================================================================
  // Type Badge Colors
  // ==========================================================================

  describe("type badge colors", () => {
    it("uses document color for document types", () => {
      render(
        <ArtifactCard {...defaultProps} artifact={createMockArtifact({ type: "prd" })} />
      );
      const badge = screen.getByTestId("artifact-type-badge");
      expect(badge).toHaveAttribute("data-category", "document");
    });

    it("uses code color for code types", () => {
      render(
        <ArtifactCard
          {...defaultProps}
          artifact={createMockArtifact({ type: "code_change" })}
        />
      );
      const badge = screen.getByTestId("artifact-type-badge");
      expect(badge).toHaveAttribute("data-category", "code");
    });

    it("uses process color for process types", () => {
      render(
        <ArtifactCard
          {...defaultProps}
          artifact={createMockArtifact({ type: "review_feedback" })}
        />
      );
      const badge = screen.getByTestId("artifact-type-badge");
      expect(badge).toHaveAttribute("data-category", "process");
    });

    it("uses context color for context types", () => {
      render(
        <ArtifactCard {...defaultProps} artifact={createMockArtifact({ type: "context" })} />
      );
      const badge = screen.getByTestId("artifact-type-badge");
      expect(badge).toHaveAttribute("data-category", "context");
    });

    it("uses log color for log types", () => {
      render(
        <ArtifactCard
          {...defaultProps}
          artifact={createMockArtifact({ type: "activity_log" })}
        />
      );
      const badge = screen.getByTestId("artifact-type-badge");
      expect(badge).toHaveAttribute("data-category", "log");
    });
  });

  // ==========================================================================
  // Accessibility
  // ==========================================================================

  describe("accessibility", () => {
    it("has button role", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.getByRole("button")).toBeInTheDocument();
    });

    it("has accessible name from artifact name", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.getByRole("button")).toHaveAccessibleName(/test prd/i);
    });

    it("indicates selected state for screen readers", () => {
      render(<ArtifactCard {...defaultProps} isSelected />);
      expect(screen.getByRole("button")).toHaveAttribute("aria-pressed", "true");
    });

    it("is focusable", () => {
      render(<ArtifactCard {...defaultProps} />);
      const card = screen.getByRole("button");
      card.focus();
      expect(card).toHaveFocus();
    });
  });

  // ==========================================================================
  // Styling
  // ==========================================================================

  describe("styling", () => {
    it("uses design tokens for background", () => {
      render(<ArtifactCard {...defaultProps} />);
      const card = screen.getByTestId("artifact-card");
      expect(card).toHaveStyle({ backgroundColor: "var(--bg-elevated)" });
    });

    it("uses design tokens for text colors", () => {
      render(<ArtifactCard {...defaultProps} />);
      const name = screen.getByTestId("artifact-name");
      expect(name).toHaveStyle({ color: "var(--text-primary)" });
    });

    it("uses design tokens for border", () => {
      render(<ArtifactCard {...defaultProps} />);
      const card = screen.getByTestId("artifact-card");
      expect(card.getAttribute("style")).toContain("border-color: var(--border-subtle)");
    });
  });

  // ==========================================================================
  // Content Type Indicator
  // ==========================================================================

  describe("content type indicator", () => {
    it("shows inline icon for inline content", () => {
      render(<ArtifactCard {...defaultProps} />);
      expect(screen.getByTestId("content-type-inline")).toBeInTheDocument();
    });

    it("shows file icon for file content", () => {
      render(
        <ArtifactCard
          {...defaultProps}
          artifact={createMockArtifact({
            content: { type: "file", path: "/path/to/file" },
          })}
        />
      );
      expect(screen.getByTestId("content-type-file")).toBeInTheDocument();
    });
  });
});
