/**
 * ProposalTierGroup Component Tests
 *
 * Tests for the collapsible tier section component with:
 * - Tier labels (Foundation, Core, Integration)
 * - Collapse behavior with explicit `defaultCollapsed`
 * - Controlled and uncontrolled modes
 * - Expand/collapse toggle
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { ProposalTierGroup, getTierLabel } from "./ProposalTierGroup";

// ============================================================================
// Helper Tests
// ============================================================================

describe("getTierLabel", () => {
  it("returns 'Foundation' for tier 0", () => {
    expect(getTierLabel(0)).toBe("Foundation");
  });

  it("returns 'Core' for tier 1", () => {
    expect(getTierLabel(1)).toBe("Core");
  });

  it("returns 'Integration' for tier 2", () => {
    expect(getTierLabel(2)).toBe("Integration");
  });

  it("returns 'Integration' for tier 3 and higher", () => {
    expect(getTierLabel(3)).toBe("Integration");
    expect(getTierLabel(5)).toBe("Integration");
    expect(getTierLabel(10)).toBe("Integration");
  });
});

// ============================================================================
// Component Tests
// ============================================================================

describe("ProposalTierGroup", () => {
  const defaultProps = {
    tier: 0,
    proposalCount: 3,
    children: <div data-testid="tier-content">Proposal content</div>,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  describe("rendering", () => {
    it("renders the tier group container", () => {
      render(<ProposalTierGroup {...defaultProps} />);
      expect(screen.getByTestId("proposal-tier-group-0")).toBeInTheDocument();
    });

    it("renders tier number", () => {
      render(<ProposalTierGroup {...defaultProps} tier={1} />);
      expect(screen.getByText("Tier 1")).toBeInTheDocument();
    });

    it("renders computed tier label for tier 0", () => {
      render(<ProposalTierGroup {...defaultProps} tier={0} />);
      expect(screen.getByText("Foundation")).toBeInTheDocument();
    });

    it("renders computed tier label for tier 1", () => {
      render(<ProposalTierGroup {...defaultProps} tier={1} />);
      expect(screen.getByText("Core")).toBeInTheDocument();
    });

    it("renders computed tier label for tier 2+", () => {
      render(<ProposalTierGroup {...defaultProps} tier={2} />);
      expect(screen.getByText("Integration")).toBeInTheDocument();
    });

    it("renders custom label when provided", () => {
      render(<ProposalTierGroup {...defaultProps} label="Custom Label" />);
      expect(screen.getByText("Custom Label")).toBeInTheDocument();
      expect(screen.queryByText("Foundation")).not.toBeInTheDocument();
    });

    it("renders proposal count when one proposal is present", () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={1} />);
      expect(screen.getByText("1")).toBeInTheDocument();
    });

    it("renders proposal count for multiple proposals", () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={5} />);
      expect(screen.getByText("5")).toBeInTheDocument();
    });

    it("renders children content when expanded", () => {
      render(<ProposalTierGroup {...defaultProps} />);
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Expand/Collapse Behavior Tests
  // ============================================================================

  describe("expand/collapse behavior", () => {
    it("starts expanded by default", () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={4} />);
      expect(screen.getByTestId("tier-content")).toBeVisible();
    });

    it("stays expanded with larger proposal counts unless explicitly collapsed", async () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={5} />);
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });

    it("stays expanded when proposalCount is 7 by default", async () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={7} />);
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });

    it("respects defaultCollapsed=true override", () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={2} defaultCollapsed={true} />);
      expect(screen.queryByTestId("tier-content")).not.toBeInTheDocument();
    });

    it("respects defaultCollapsed=false override", () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={10} defaultCollapsed={false} />);
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });

    it("toggles content visibility when header is clicked", async () => {
      const user = userEvent.setup();
      render(<ProposalTierGroup {...defaultProps} proposalCount={3} />);

      // Initially expanded by default
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();

      // Click to collapse
      const header = screen.getByRole("button");
      await user.click(header);
      expect(screen.queryByTestId("tier-content")).not.toBeInTheDocument();

      // Click to expand again
      await user.click(header);
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });

    it("expands explicitly collapsed tier when clicked", async () => {
      const user = userEvent.setup();
      render(<ProposalTierGroup {...defaultProps} proposalCount={6} defaultCollapsed={true} />);

      // Initially collapsed
      expect(screen.queryByTestId("tier-content")).not.toBeInTheDocument();

      // Click to expand
      const header = screen.getByRole("button");
      await user.click(header);
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Controlled Mode Tests
  // ============================================================================

  describe("controlled mode", () => {
    it("uses isExpanded prop when provided", () => {
      render(
        <ProposalTierGroup {...defaultProps} isExpanded={true} proposalCount={10} />
      );
      // Should be expanded when controlled by parent
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });

    it("respects isExpanded=false prop", () => {
      render(
        <ProposalTierGroup {...defaultProps} isExpanded={false} proposalCount={2} />
      );
      // Should be collapsed despite low count
      expect(screen.queryByTestId("tier-content")).not.toBeInTheDocument();
    });

    it("calls onExpandedChange when toggled", async () => {
      const user = userEvent.setup();
      const onExpandedChange = vi.fn();

      render(
        <ProposalTierGroup
          {...defaultProps}
          isExpanded={true}
          onExpandedChange={onExpandedChange}
        />
      );

      const header = screen.getByRole("button");
      await user.click(header);

      expect(onExpandedChange).toHaveBeenCalledWith(false);
    });

    it("does not change state internally when controlled", async () => {
      const user = userEvent.setup();
      const onExpandedChange = vi.fn();

      const { rerender } = render(
        <ProposalTierGroup
          {...defaultProps}
          isExpanded={true}
          onExpandedChange={onExpandedChange}
        />
      );

      // Click to try to collapse
      const header = screen.getByRole("button");
      await user.click(header);

      // Callback called but state hasn't changed (controlled)
      expect(onExpandedChange).toHaveBeenCalledWith(false);
      // Content still visible because isExpanded is still true
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();

      // Parent updates the prop
      rerender(
        <ProposalTierGroup
          {...defaultProps}
          isExpanded={false}
          onExpandedChange={onExpandedChange}
        />
      );

      // Now content should be hidden
      expect(screen.queryByTestId("tier-content")).not.toBeInTheDocument();
    });
  });

  // ============================================================================
  // Styling Tests
  // ============================================================================

  describe("styling", () => {
    it("applies custom className", () => {
      render(<ProposalTierGroup {...defaultProps} className="custom-class" />);
      expect(screen.getByTestId("proposal-tier-group-0")).toHaveClass("custom-class");
    });

    it("renders chevron icon", () => {
      render(<ProposalTierGroup {...defaultProps} />);
      // Chevron should be present in the button
      const button = screen.getByRole("button");
      expect(button.querySelector("svg")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Edge Cases
  // ============================================================================

  describe("edge cases", () => {
    it("handles zero proposals", () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={0} />);
      expect(screen.getByText("0")).toBeInTheDocument();
    });

    it("handles large tier numbers", () => {
      render(<ProposalTierGroup {...defaultProps} tier={99} />);
      expect(screen.getByText("Tier 99")).toBeInTheDocument();
      expect(screen.getByText("Integration")).toBeInTheDocument();
    });

    it("handles large proposal counts", () => {
      render(<ProposalTierGroup {...defaultProps} proposalCount={1000} />);
      expect(screen.getByText("1000")).toBeInTheDocument();
      expect(screen.getByTestId("tier-content")).toBeInTheDocument();
    });
  });
});
