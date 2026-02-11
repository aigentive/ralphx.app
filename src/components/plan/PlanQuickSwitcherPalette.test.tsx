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
 * - Scroll behavior when navigating with keyboard
 * - Focus ring visibility on highlighted items
 * - Home/End key navigation
 * - Edge cases (empty list, wrapping)
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
  const mockClearActivePlan = vi.fn();
  const mockOnClose = vi.fn();

  const defaultProps = {
    projectId: "project-1",
    isOpen: true,
    onClose: mockOnClose,
    showClearAction: false,
  };

  const defaultStoreState = {
    activePlanByProject: { "project-1": "session-1" },
    planCandidates: mockCandidates,
    isLoading: false,
    error: null,
    loadCandidates: mockLoadCandidates,
    setActivePlan: mockSetActivePlan,
    clearActivePlan: mockClearActivePlan,
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
      expect(screen.getByText(/5 of 10 incomplete/)).toBeInTheDocument();
      expect(screen.getByText(/2 of 5 incomplete/)).toBeInTheDocument();
      expect(screen.getByText(/3 of 8 incomplete/)).toBeInTheDocument();
    });

    it("shows active work indicator for plans with activeNow > 0", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const featureAStats = screen.getByText(/5 of 10 incomplete/);
      expect(featureAStats.textContent).toContain("Active work");

      const bugFixesStats = screen.getByText(/3 of 8 incomplete/);
      expect(bugFixesStats.textContent).toContain("Active work");

      const featureBStats = screen.getByText(/2 of 5 incomplete/);
      expect(featureBStats.textContent).not.toContain("Active work");
    });

    it("shows a complete summary when there are no incomplete tasks", () => {
      const allDoneCandidates = [
        createMockCandidate({
          sessionId: "session-4",
          title: "All Done Plan",
          taskStats: { total: 7, incomplete: 0, activeNow: 0 },
        }),
      ];
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({ ...defaultStoreState, planCandidates: allDoneCandidates })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      expect(screen.getByText("7 tasks complete")).toBeInTheDocument();
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
      expect(featureBButton?.className).toMatch(/bg-accent/);
    });

    it("navigates up with ArrowUp and clamps at top", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Press up from index 0, should remain at index 0
      fireEvent.keyDown(input, { key: "ArrowUp" });

      const featureAButton = screen.getByText("Feature A").closest("button");
      expect(featureAButton?.className).toMatch(/bg-accent|bg-accent\/50/);
    });

    it("clamps navigation at list boundaries", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Stay at top when going up
      fireEvent.keyDown(input, { key: "ArrowUp" });
      const featureAButton = screen.getByText("Feature A").closest("button");
      expect(featureAButton?.className).toMatch(/bg-accent|bg-accent\/50/);

      // Navigate down to last item
      fireEvent.keyDown(input, { key: "ArrowDown" });
      fireEvent.keyDown(input, { key: "ArrowDown" });
      const bugFixesButton = screen.getByText("Bug Fixes").closest("button");
      expect(bugFixesButton?.className).toMatch(/bg-accent/);

      // Attempt to go past last item; should remain on last
      fireEvent.keyDown(input, { key: "ArrowDown" });
      expect(bugFixesButton?.className).toMatch(/bg-accent/);
    });

    it("jumps to first and last with Shift+ArrowUp/Down", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Move highlight away from first
      fireEvent.keyDown(input, { key: "ArrowDown" }); // index 1

      // Shift+ArrowDown should jump to last
      fireEvent.keyDown(input, { key: "ArrowDown", shiftKey: true });
      const bugFixesButton = screen.getByText("Bug Fixes").closest("button");
      expect(bugFixesButton?.className).toMatch(/bg-accent/);

      // Shift+ArrowUp should jump to first
      fireEvent.keyDown(input, { key: "ArrowUp", shiftKey: true });
      const featureAButton = screen.getByText("Feature A").closest("button");
      expect(featureAButton?.className).toMatch(/bg-accent|bg-accent\/50/);
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
      expect(featureAButton?.className).toMatch(/bg-accent|bg-accent\/50/);
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

      expect(featureBButton?.className).toMatch(/bg-accent/);
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

    it("uses provided selection source when selecting a plan", async () => {
      mockSetActivePlan.mockResolvedValue(undefined);

      render(
        <PlanQuickSwitcherPalette
          {...defaultProps}
          selectionSource="graph_inline"
        />
      );

      const featureBButton = screen.getByText("Feature B").closest("button")!;
      fireEvent.click(featureBButton);

      await waitFor(() => {
        expect(mockSetActivePlan).toHaveBeenCalledWith("project-1", "session-2", "graph_inline");
      });
    });

    it("shows and executes clear action when enabled", async () => {
      mockClearActivePlan.mockResolvedValue(undefined);

      render(
        <PlanQuickSwitcherPalette
          {...defaultProps}
          showClearAction
        />
      );

      const clearButton = screen.getByTestId("plan-quick-switcher-clear");
      fireEvent.click(clearButton);

      await waitFor(() => {
        expect(mockClearActivePlan).toHaveBeenCalledWith("project-1");
        expect(mockOnClose).toHaveBeenCalled();
      });
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

  // ==========================================================================
  // Animation & Performance
  // ==========================================================================

  describe("animations", () => {
    it("applies correct initial animation properties", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const palette = screen.getByPlaceholderText(/Search plans/).closest(".fixed");
      expect(palette).toHaveClass("fixed", "top-20", "left-1/2", "-translate-x-1/2", "z-50");
    });

    it("applies transition classes to candidate items", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const featureAButton = screen.getByText("Feature A").closest("button")!;
      expect(featureAButton).toHaveClass("transition-all", "origin-center");
    });

    it("applies hover scale class to candidate items", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const featureAButton = screen.getByText("Feature A").closest("button")!;
      expect(featureAButton.className).toMatch(/hover:scale-\[1\.01\]/);
    });

    it("applies transition-colors to search input", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const input = screen.getByPlaceholderText(/Search plans/);
      expect(input).toHaveClass("transition-colors");
    });

    it("applies transition-colors to loading state", () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({ ...defaultStoreState, isLoading: true })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const loadingDiv = screen.getByText("Loading plans...").closest("div")!;
      expect(loadingDiv).toHaveClass("transition-colors");
    });

    it("applies transition-colors to empty state", () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({ ...defaultStoreState, planCandidates: [] })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const emptyDiv = screen.getByText("No accepted plans found").closest("div")!;
      expect(emptyDiv).toHaveClass("transition-colors");
    });

    it("uses fixed width to prevent layout shifts", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const palette = screen.getByPlaceholderText(/Search plans/).closest(".fixed");
      expect(palette?.className).toMatch(/w-\[420px\]/);
    });

    it("constrains scroll area height to prevent layout shifts", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      // ScrollArea should have max-h constraint
      const scrollArea = screen.getByText("Feature A").closest("[class*='max-h']");
      expect(scrollArea).toBeTruthy();
    });
  });

  // ==========================================================================
  // Keyboard Navigation Polish (scroll, focus ring, Home/End, edge cases)
  // ==========================================================================

  describe("scroll behavior", () => {
    it("should scroll highlighted item into view when navigating down", async () => {
      const user = userEvent.setup();
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const input = screen.getByPlaceholderText(/Search plans/);
      await user.click(input);

      // Mock scrollIntoView
      const scrollIntoViewMock = vi.fn();
      Element.prototype.scrollIntoView = scrollIntoViewMock;

      // Navigate down
      await user.keyboard("{ArrowDown}");

      await waitFor(() => {
        expect(scrollIntoViewMock).toHaveBeenCalledWith({
          block: "nearest",
          behavior: "smooth",
        });
      });
    });

    it("should scroll highlighted item into view when navigating up", async () => {
      const user = userEvent.setup();
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const input = screen.getByPlaceholderText(/Search plans/);
      await user.click(input);

      const scrollIntoViewMock = vi.fn();
      Element.prototype.scrollIntoView = scrollIntoViewMock;

      // Move down first so ArrowUp changes the highlighted item.
      await user.keyboard("{ArrowDown}");
      await user.keyboard("{ArrowUp}");

      await waitFor(() => {
        expect(scrollIntoViewMock).toHaveBeenCalledWith({
          block: "nearest",
          behavior: "smooth",
        });
      });
    });
  });

  describe("Home/End key navigation", () => {
    it("should jump to first item when Home key is pressed", async () => {
      const user = userEvent.setup();
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const input = screen.getByPlaceholderText(/Search plans/);
      await user.click(input);

      // Navigate down a few times
      await user.keyboard("{ArrowDown}");
      await user.keyboard("{ArrowDown}");

      // Press Home
      await user.keyboard("{Home}");

      // First item should be highlighted
      const firstButton = screen.getByText("Feature A").closest("button");
      expect(firstButton?.className).toMatch(/bg-accent|bg-accent\/50/);
    });

    it("should jump to last item when End key is pressed", async () => {
      const user = userEvent.setup();
      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const input = screen.getByPlaceholderText(/Search plans/);
      await user.click(input);

      // Press End
      await user.keyboard("{End}");

      // Last item should be highlighted
      const lastButton = screen.getByText("Bug Fixes").closest("button");
      expect(lastButton?.className).toMatch(/bg-accent/);
    });
  });

  describe("empty list edge case", () => {
    it("should handle empty list gracefully with navigation keys", async () => {
      const user = userEvent.setup();

      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({ ...defaultStoreState, planCandidates: [] })
      );

      render(<PlanQuickSwitcherPalette {...defaultProps} />);

      const input = screen.getByPlaceholderText(/Search plans/);
      await user.click(input);

      // Navigation keys should not throw errors
      await user.keyboard("{ArrowDown}");
      await user.keyboard("{ArrowUp}");
      await user.keyboard("{Home}");
      await user.keyboard("{End}");

      expect(screen.getByText("No accepted plans found")).toBeInTheDocument();
    });
  });

  describe("focus ring accessibility", () => {
    it("should show focus ring on highlighted item", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // Navigate down
      fireEvent.keyDown(input, { key: "ArrowDown" });

      // Second item should have focus ring
      const featureBButton = screen.getByText("Feature B").closest("button");
      expect(featureBButton).toHaveClass("focus-visible:ring-1");
      expect(featureBButton?.className).toMatch(/bg-accent/);
    });

    it("should move focus ring when navigating", () => {
      render(<PlanQuickSwitcherPalette {...defaultProps} />);
      const input = screen.getByPlaceholderText(/Search plans/);

      // First item should be highlighted initially
      let highlightedButton = screen.getByText("Feature A").closest("button");
      expect(highlightedButton).toHaveClass("focus-visible:ring-1");

      // Navigate down
      fireEvent.keyDown(input, { key: "ArrowDown" });

      // First item remains active (bg-accent/50) but is no longer the highlighted row

      // Second item should have ring
      highlightedButton = screen.getByText("Feature B").closest("button");
      expect(highlightedButton?.className).toMatch(/bg-accent/);
    });
  });
});
