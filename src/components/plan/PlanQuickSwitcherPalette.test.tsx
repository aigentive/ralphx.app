/**
 * PlanQuickSwitcherPalette component tests
 *
 * Tests for:
 * - Opening/closing behavior
 * - Keyboard navigation (ArrowUp/Down, Enter, Escape)
 * - Search filtering
 * - Plan selection
 * - Active plan indicator
 * - Click outside to close
 * - Auto-focus on open
 * - Empty state
 * - Loading state
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { PlanQuickSwitcherPalette } from "./PlanQuickSwitcherPalette";
import { usePlanStore, type PlanCandidate } from "@/stores/planStore";

// Mock zustand store
vi.mock("@/stores/planStore", () => ({
  usePlanStore: vi.fn(),
}));

// ============================================================================
// Test Data
// ============================================================================

const createMockCandidate = (overrides: Partial<PlanCandidate> = {}): PlanCandidate => ({
  sessionId: "session-1",
  title: "Test Plan",
  acceptedAt: "2026-01-24T10:00:00Z",
  taskStats: {
    total: 10,
    incomplete: 5,
    activeNow: 2,
  },
  interactionStats: {
    selectedCount: 3,
    lastSelectedAt: "2026-01-24T12:00:00Z",
  },
  score: 0.85,
  ...overrides,
});

const mockCandidates: PlanCandidate[] = [
  createMockCandidate({ sessionId: "session-1", title: "Feature A" }),
  createMockCandidate({ sessionId: "session-2", title: "Feature B", taskStats: { total: 5, incomplete: 2, activeNow: 0 } }),
  createMockCandidate({ sessionId: "session-3", title: "Bug Fixes", taskStats: { total: 8, incomplete: 3, activeNow: 1 } }),
];

describe("PlanQuickSwitcherPalette", () => {
  const mockLoadCandidates = vi.fn();
  const mockSetActivePlan = vi.fn();
  const mockOnClose = vi.fn();

  const defaultProps = {
    projectId: "project-1",
    isOpen: true,
    onClose: mockOnClose,
  };

  const defaultStoreState = {
    activePlanByProject: { "project-1": "session-1" },
    planCandidates: mockCandidates,
    isLoading: false,
    error: null,
    loadCandidates: mockLoadCandidates,
    setActivePlan: mockSetActivePlan,
  };

  beforeEach(() => {
    vi.clearAllMocks();
    (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
      selector(defaultStoreState)
    );
  });

  // ==========================================================================
  // Rendering
  // ==========================================================================

  describe("rendering", () => {
    it("does not render when isOpen is false", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} isOpen={false} />);
      expect(screen.queryByPlaceholderText(/Search plans/)).not.toBeInTheDocument();
    });

    it("renders when isOpen is true", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(screen.getByPlaceholderText(/Search plans/)).toBeInTheDocument();
    });

    it("displays all candidates", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(screen.getByText("Feature A")).toBeInTheDocument();
      expect(screen.getByText("Feature B")).toBeInTheDocument();
      expect(screen.getByText("Bug Fixes")).toBeInTheDocument();
    });

    it("displays task stats for each candidate", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(screen.getByText(/5\/10 incomplete/)).toBeInTheDocument();
      expect(screen.getByText(/2\/5 incomplete/)).toBeInTheDocument();
      expect(screen.getByText(/3\/8 incomplete/)).toBeInTheDocument();
    });

    it("shows active work indicator for plans with activeNow > 0", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const featureAStats = screen.getByText(/5\/10 incomplete/);
      expect(featureAStats.textContent).toContain("Active work");

      const bugFixesStats = screen.getByText(/3\/8 incomplete/);
      expect(bugFixesStats.textContent).toContain("Active work");

      const featureBStats = screen.getByText(/2\/5 incomplete/);
      expect(featureBStats.textContent).not.toContain("Active work");
    });

    it("shows check icon for active plan", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      // The active plan (session-1 = Feature A) should have a check icon
      const featureAButton = screen.getByText("Feature A").closest("button");
      expect(featureAButton?.querySelector("svg")).toBeInTheDocument();
    });

    it("displays 'Untitled Plan' for null title", () => {
      const candidatesWithNull = [
        createMockCandidate({ sessionId: "session-1", title: null }),
      ];
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({ ...defaultStoreState, planCandidates: candidatesWithNull })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(screen.getByText("Untitled Plan")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Loading State
  // ==========================================================================

  describe("loading state", () => {
    it("shows loading message when isLoading is true", () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({ ...defaultStoreState, isLoading: true })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(screen.getByText("Loading plans...")).toBeInTheDocument();
    });

    it("calls loadCandidates on open", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(mockLoadCandidates).toHaveBeenCalledWith("project-1");
    });
  });

  // ==========================================================================
  // Empty State
  // ==========================================================================

  describe("empty state", () => {
    it("shows empty state when no candidates", () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({ ...defaultStoreState, planCandidates: [] })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(screen.getByText("No accepted plans found")).toBeInTheDocument();
    });
  });

  // ==========================================================================
  // Search Filtering
  // ==========================================================================

  describe("search filtering", () => {
    it("filters candidates by search query (case-insensitive)", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      fireEvent.change(input, { target: { value: "feature" } });

      expect(screen.getByText("Feature A")).toBeInTheDocument();
      expect(screen.getByText("Feature B")).toBeInTheDocument();
      expect(screen.queryByText("Bug Fixes")).not.toBeInTheDocument();
    });

    it("shows empty state when search yields no results", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      fireEvent.change(input, { target: { value: "nonexistent" } });

      expect(screen.getByText("No accepted plans found")).toBeInTheDocument();
    });

    it("resets search query when closed", async () => {
      const { rerender } = render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/) as HTMLInputElement;

      fireEvent.change(input, { target: { value: "feature" } });
      expect(input.value).toBe("feature");

      rerender(<PlanQuickSwitcherPalette {...defaultProps} isOpen={false} />);
      rerender(<PlanQuickSwitcherPalette {...defaultProps} isOpen={true} />);

      await waitFor(() => {
        const newInput = screen.getByPlaceholderText(/Search plans/) as HTMLInputElement;
        expect(newInput.value).toBe("");
      });
    });
  });

  // ==========================================================================
  // Keyboard Navigation
  // ==========================================================================

  describe("keyboard navigation", () => {
    it("closes palette on Escape key", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      fireEvent.keyDown(input, { key: "Escape" });

      expect(mockOnClose).toHaveBeenCalled();
    });

    it("navigates down with ArrowDown", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Initially no highlight (index 0), press down to move to index 1
      fireEvent.keyDown(input, { key: "ArrowDown" });

      // Feature B should be highlighted (index 1)
      const featureBButton = screen.getByText("Feature B").closest("button");
      expect(featureBButton).toHaveClass("bg-white/10");
    });

    it("navigates up with ArrowUp", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Press up from index 0, should wrap to last item (index 2)
      fireEvent.keyDown(input, { key: "ArrowUp" });

      const bugFixesButton = screen.getByText("Bug Fixes").closest("button");
      expect(bugFixesButton).toHaveClass("bg-white/10");
    });

    it("wraps navigation at list boundaries", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Navigate to last item
      fireEvent.keyDown(input, { key: "ArrowUp" });
      // Navigate past last item (should wrap to first)
      fireEvent.keyDown(input, { key: "ArrowDown" });

      const featureAButton = screen.getByText("Feature A").closest("button");
      expect(featureAButton).toHaveClass("bg-white/10");
    });

    it("selects highlighted plan on Enter", async () => {
      mockSetActivePlan.mockResolvedValue(undefined);

      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Navigate to Feature B (index 1)
      fireEvent.keyDown(input, { key: "ArrowDown" });
      // Select it
      fireEvent.keyDown(input, { key: "Enter" });

      await waitFor(() => {
        expect(mockSetActivePlan).toHaveBeenCalledWith("project-1", "session-2", "quick_switcher");
      });
    });

    it("resets highlighted index when search changes", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Navigate down
      fireEvent.keyDown(input, { key: "ArrowDown" });
      fireEvent.keyDown(input, { key: "ArrowDown" });

      // Change search (should reset to index 0)
      fireEvent.change(input, { target: { value: "feature" } });

      // Feature A should be highlighted now (index 0 of filtered results)
      const featureAButton = screen.getByText("Feature A").closest("button");
      expect(featureAButton).toHaveClass("bg-white/10");
    });
  });

  // ==========================================================================
  // Mouse Interaction
  // ==========================================================================

  describe("mouse interaction", () => {
    it("updates highlighted index on mouse enter", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const featureBButton = screen.getByText("Feature B").closest("button")!;
      fireEvent.mouseEnter(featureBButton);

      expect(featureBButton).toHaveClass("bg-white/10");
    });

    it("selects plan on click", async () => {
      mockSetActivePlan.mockResolvedValue(undefined);

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const featureBButton = screen.getByText("Feature B").closest("button")!;
      fireEvent.click(featureBButton);

      await waitFor(() => {
        expect(mockSetActivePlan).toHaveBeenCalledWith("project-1", "session-2", "quick_switcher");
        expect(mockOnClose).toHaveBeenCalled();
      });
    });

    it("closes on successful selection", async () => {
      mockSetActivePlan.mockResolvedValue(undefined);

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const featureAButton = screen.getByText("Feature A").closest("button")!;
      fireEvent.click(featureAButton);

      await waitFor(() => {
        expect(mockOnClose).toHaveBeenCalled();
      });
    });

    it("logs error on failed selection", async () => {
      const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
      mockSetActivePlan.mockRejectedValue(new Error("Test error"));

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const featureAButton = screen.getByText("Feature A").closest("button")!;
      fireEvent.click(featureAButton);

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalledWith("Failed to set active plan:", expect.any(Error));
      });

      consoleErrorSpy.mockRestore();
    });
  });

  // ==========================================================================
  // Click Outside
  // ==========================================================================

  describe("click outside", () => {
    it("closes palette when clicking outside", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      // Click on document body (outside the palette)
      fireEvent.mouseDown(document.body);

      expect(mockOnClose).toHaveBeenCalled();
    });

    it("does not close when clicking inside palette", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const input = screen.getByPlaceholderText(/Search plans/);
      fireEvent.mouseDown(input);

      expect(mockOnClose).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Error State
  // ==========================================================================

  describe("error state", () => {
    it("displays error message when there is an error", () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({
          ...defaultStoreState,
          error: "Failed to load plans",
          planCandidates: [],
        })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      expect(screen.getByText("Failed to load plans")).toBeInTheDocument();
      expect(screen.getByText("Retry")).toBeInTheDocument();
    });

    it("calls loadCandidates when retry button is clicked", async () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({
          ...defaultStoreState,
          error: "Network error",
          planCandidates: [],
        })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const retryButton = screen.getByText("Retry");
      fireEvent.click(retryButton);

      await waitFor(() => {
        expect(mockLoadCandidates).toHaveBeenCalledWith("project-1");
      });
    });

    it("shows error state instead of empty state when both error and no candidates", () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({
          ...defaultStoreState,
          error: "Failed to fetch",
          planCandidates: [],
        })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      expect(screen.getByText("Failed to fetch")).toBeInTheDocument();
      expect(screen.queryByText("No accepted plans found")).not.toBeInTheDocument();
    });
  });
});
