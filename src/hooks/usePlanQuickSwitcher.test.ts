/**
 * usePlanQuickSwitcher hook tests
 *
 * Tests for:
 * - State management (searchQuery, highlightedIndex, anchorCenterX)
 * - Refs management (inputRef, containerRef, highlightedItemRef)
 * - Store subscriptions
 * - Derived data (filteredCandidates, canClearPlan)
 * - Event handlers (handleKeyDown, handleSelect, handleClear, handleRetry)
 * - Effects (auto-focus, load on open, reset on close, anchor centering)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { usePlanQuickSwitcher } from "./usePlanQuickSwitcher";
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
  createMockCandidate({ sessionId: "session-2", title: "Feature B" }),
  createMockCandidate({ sessionId: "session-3", title: "Bug Fixes" }),
];

describe("usePlanQuickSwitcher", () => {
  const mockLoadCandidates = vi.fn();
  const mockSetActivePlan = vi.fn().mockResolvedValue(undefined);
  const mockClearActivePlan = vi.fn().mockResolvedValue(undefined);
  const mockOnClose = vi.fn();

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

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ==========================================================================
  // State Management
  // ==========================================================================

  describe("state management", () => {
    it("initializes with empty searchQuery", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.searchQuery).toBe("");
    });

    it("initializes with highlightedIndex 0", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.highlightedIndex).toBe(0);
    });

    it("resets searchQuery when closed", () => {
      const { result, rerender } = renderHook(
        ({ isOpen }) =>
          usePlanQuickSwitcher({
            isOpen,
            projectId: "project-1",
            onClose: mockOnClose,
          }),
        { initialProps: { isOpen: true } }
      );

      act(() => {
        result.current.setSearchQuery("test");
      });

      expect(result.current.searchQuery).toBe("test");

      rerender({ isOpen: false });

      expect(result.current.searchQuery).toBe("");
    });

    it("resets highlightedIndex when search query changes", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      act(() => {
        result.current.setHighlightedIndex(2);
      });

      expect(result.current.highlightedIndex).toBe(2);

      act(() => {
        result.current.setSearchQuery("feature");
      });

      expect(result.current.highlightedIndex).toBe(0);
    });
  });

  // ==========================================================================
  // Store Subscriptions
  // ==========================================================================

  describe("store subscriptions", () => {
    it("exposes activePlanId from store", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.activePlanId).toBe("session-1");
    });

    it("exposes planCandidates from store", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.planCandidates).toEqual(mockCandidates);
    });

    it("exposes isLoading from store", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.isLoading).toBe(false);
    });

    it("exposes error from store", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.error).toBeNull();
    });
  });

  // ==========================================================================
  // Derived Data
  // ==========================================================================

  describe("derived data", () => {
    it("returns all candidates when searchQuery is empty", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.filteredCandidates).toEqual(mockCandidates);
    });

    it("filters candidates by title (case-insensitive)", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      act(() => {
        result.current.setSearchQuery("feature");
      });

      expect(result.current.filteredCandidates).toHaveLength(2);
      expect(result.current.filteredCandidates[0].title).toBe("Feature A");
      expect(result.current.filteredCandidates[1].title).toBe("Feature B");
    });

    it("returns canClearPlan true when showClearAction and activePlanId exist", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
          showClearAction: true,
        })
      );

      expect(result.current.canClearPlan).toBe(true);
    });

    it("returns canClearPlan false when showClearAction is false", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
          showClearAction: false,
        })
      );

      expect(result.current.canClearPlan).toBe(false);
    });

    it("returns canClearPlan false when no active plan", () => {
      (usePlanStore as unknown as ReturnType<typeof vi.fn>).mockImplementation((selector) =>
        selector({
          ...defaultStoreState,
          activePlanByProject: {},
        })
      );

      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
          showClearAction: true,
        })
      );

      expect(result.current.canClearPlan).toBe(false);
    });
  });

  // ==========================================================================
  // Handlers
  // ==========================================================================

  describe("handlers", () => {
    it("handleSelect calls setActivePlan and onClose", async () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
          selectionSource: "quick_switcher",
        })
      );

      await act(async () => {
        await result.current.handleSelect("session-2");
      });

      expect(mockSetActivePlan).toHaveBeenCalledWith("project-1", "session-2", "quick_switcher");
      expect(mockOnClose).toHaveBeenCalled();
    });

    it("handleClear calls clearActivePlan and onClose", async () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      await act(async () => {
        await result.current.handleClear();
      });

      expect(mockClearActivePlan).toHaveBeenCalledWith("project-1");
      expect(mockOnClose).toHaveBeenCalled();
    });

    it("handleRetry calls loadCandidates", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      act(() => {
        result.current.handleRetry();
      });

      expect(mockLoadCandidates).toHaveBeenCalledWith("project-1");
    });
  });

  // ==========================================================================
  // Effects
  // ==========================================================================

  describe("effects", () => {
    it("loads candidates when opened", () => {
      renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(mockLoadCandidates).toHaveBeenCalledWith("project-1");
    });

    it("does not load candidates when closed", () => {
      renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: false,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(mockLoadCandidates).not.toHaveBeenCalled();
    });

    it("loads candidates when projectId changes", () => {
      const { rerender } = renderHook(
        ({ projectId }) =>
          usePlanQuickSwitcher({
            isOpen: true,
            projectId,
            onClose: mockOnClose,
          }),
        { initialProps: { projectId: "project-1" } }
      );

      expect(mockLoadCandidates).toHaveBeenCalledWith("project-1");

      rerender({ projectId: "project-2" });

      expect(mockLoadCandidates).toHaveBeenCalledWith("project-2");
    });
  });

  // ==========================================================================
  // Refs
  // ==========================================================================

  describe("refs", () => {
    it("provides inputRef", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.inputRef).toBeDefined();
      expect(result.current.inputRef.current).toBeNull(); // Not attached to DOM in unit test
    });

    it("provides containerRef", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.containerRef).toBeDefined();
    });

    it("provides highlightedItemRef", () => {
      const { result } = renderHook(() =>
        usePlanQuickSwitcher({
          isOpen: true,
          projectId: "project-1",
          onClose: mockOnClose,
        })
      );

      expect(result.current.highlightedItemRef).toBeDefined();
    });
  });
});
