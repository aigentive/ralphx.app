/**
 * usePlanQuickSwitcher - Orchestration hook for plan quick switcher palette
 *
 * Extracts all state, effects, and handlers from PlanQuickSwitcherPalette.
 * Owns:
 * - State: searchQuery, highlightedIndex, anchorCenterX
 * - Refs: inputRef, containerRef, highlightedItemRef
 * - Store subscriptions: activePlanId, planCandidates, isLoading, error, actions
 * - Derived data: filteredCandidates, canClearPlan
 * - Effects: auto-focus, load on open, reset on close, highlight reset, anchor centering, click-outside
 * - Handlers: handleKeyDown, handleSelect, handleClear, handleRetry, setters
 */

import { useState, useEffect, useRef, useCallback } from "react";
import { usePlanStore } from "@/stores/planStore";
import type { PlanCandidate } from "@/stores/planStore";
import type { SelectionSource } from "@/api/plan";
import { usePlanCandidateSort } from "./usePlanCandidateSort";

// ============================================================================
// Types
// ============================================================================

export interface UsePlanQuickSwitcherProps {
  projectId: string;
  isOpen: boolean;
  onClose: () => void;
  /** Source attribution for selection analytics */
  selectionSource?: SelectionSource;
  /** Show clear active plan command at top of list when active plan exists */
  showClearAction?: boolean;
  /** Optional CSS selector used to anchor horizontal centering to a specific container */
  anchorSelector?: string;
}

export interface UsePlanQuickSwitcherReturn {
  // State
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  highlightedIndex: number;
  setHighlightedIndex: (index: number | ((prev: number) => number)) => void;
  anchorCenterX: number | null;

  // Refs
  inputRef: React.RefObject<HTMLInputElement | null>;
  containerRef: React.RefObject<HTMLDivElement | null>;
  highlightedItemRef: React.RefObject<HTMLButtonElement | null>;

  // Store state
  activePlanId: string | null;
  planCandidates: PlanCandidate[];
  isLoading: boolean;
  error: string | null;

  // Derived data
  filteredCandidates: PlanCandidate[];
  canClearPlan: boolean;

  // Handlers
  handleKeyDown: (e: React.KeyboardEvent) => void;
  handleSelect: (sessionId: string) => Promise<void>;
  handleClear: () => Promise<void>;
  handleRetry: () => void;
}

// ============================================================================
// Hook Implementation
// ============================================================================

export function usePlanQuickSwitcher({
  projectId,
  isOpen,
  onClose,
  selectionSource = "quick_switcher",
  showClearAction = true,
  anchorSelector,
}: UsePlanQuickSwitcherProps): UsePlanQuickSwitcherReturn {
  // State
  const [searchQuery, setSearchQuery] = useState("");
  const [highlightedIndex, setHighlightedIndex] = useState(0);
  const [anchorCenterX, setAnchorCenterX] = useState<number | null>(null);

  // Refs
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const highlightedItemRef = useRef<HTMLButtonElement>(null);

  // Store subscriptions
  const activePlanId = usePlanStore((state) => state.activePlanByProject[projectId] ?? null);
  const planCandidates = usePlanStore((state) => state.planCandidates);
  const isLoading = usePlanStore((state) => state.isLoading);
  const error = usePlanStore((state) => state.error);
  const loadCandidates = usePlanStore((state) => state.loadCandidates);
  const setActivePlan = usePlanStore((state) => state.setActivePlan);
  const clearActivePlan = usePlanStore((state) => state.clearActivePlan);

  // Derived data: planCandidates → sortedCandidates → filteredCandidates
  const sortedCandidates = usePlanCandidateSort(planCandidates);

  const filteredCandidates = searchQuery
    ? sortedCandidates.filter((plan) =>
        (plan.title || "Untitled Plan").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : sortedCandidates;

  const canClearPlan = showClearAction && Boolean(activePlanId);

  // Handlers
  const handleSelect = useCallback(
    async (sessionId: string) => {
      try {
        await setActivePlan(projectId, sessionId, selectionSource);
        onClose();
      } catch (error) {
        console.error("Failed to set active plan:", error);
      }
    },
    [projectId, setActivePlan, onClose, selectionSource]
  );

  const handleClear = useCallback(async () => {
    try {
      await clearActivePlan(projectId);
      onClose();
    } catch (error) {
      console.error("Failed to clear active plan:", error);
    }
  }, [clearActivePlan, onClose, projectId]);

  const handleRetry = useCallback(() => {
    loadCandidates(projectId);
  }, [projectId, loadCandidates]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      const candidateCount = filteredCandidates.length;
      const itemCount = candidateCount + (canClearPlan ? 1 : 0);

      // Prevent navigation if no interactive rows
      if (itemCount === 0 && ["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) {
        return;
      }

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          if (e.shiftKey) {
            setHighlightedIndex(itemCount - 1);
          } else {
            setHighlightedIndex((i) => Math.min(i + 1, itemCount - 1));
          }
          break;
        case "ArrowUp":
          e.preventDefault();
          if (e.shiftKey) {
            setHighlightedIndex(0);
          } else {
            setHighlightedIndex((i) => Math.max(i - 1, 0));
          }
          break;
        case "Home":
          e.preventDefault();
          setHighlightedIndex(0);
          break;
        case "End":
          e.preventDefault();
          setHighlightedIndex(itemCount - 1);
          break;
        case "Enter": {
          e.preventDefault();
          if (canClearPlan && highlightedIndex === 0) {
            handleClear();
            return;
          }

          const candidateIndex = canClearPlan ? highlightedIndex - 1 : highlightedIndex;
          if (candidateIndex >= 0 && filteredCandidates[candidateIndex]) {
            handleSelect(filteredCandidates[candidateIndex].sessionId);
          }
          break;
        }
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [canClearPlan, filteredCandidates, handleClear, highlightedIndex, onClose, handleSelect]
  );

  // Effects

  // Auto-focus search input when opened
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);

  // Load candidates when opened
  useEffect(() => {
    if (isOpen) {
      loadCandidates(projectId);
    }
  }, [isOpen, projectId, loadCandidates]);

  // Reset state when closed
  useEffect(() => {
    if (!isOpen) {
      setSearchQuery("");
      setHighlightedIndex(0);
    }
  }, [isOpen]);

  // Reset highlighted index when filtered list changes
  useEffect(() => {
    setHighlightedIndex(0);
  }, [searchQuery]);

  // Scroll highlighted item into view
  useEffect(() => {
    if (
      highlightedItemRef.current &&
      typeof highlightedItemRef.current.scrollIntoView === "function"
    ) {
      highlightedItemRef.current.scrollIntoView({
        block: "nearest",
        behavior: "smooth",
      });
    }
  }, [highlightedIndex]);

  // Center to the requested anchor container (e.g., split-layout left pane).
  useEffect(() => {
    if (!isOpen) return;

    const updateAnchorCenter = () => {
      if (!anchorSelector) {
        setAnchorCenterX(null);
        return;
      }
      const anchor = document.querySelector(anchorSelector);
      if (anchor instanceof HTMLElement) {
        const rect = anchor.getBoundingClientRect();
        setAnchorCenterX(rect.left + rect.width / 2);
      } else {
        setAnchorCenterX(null);
      }
    };

    updateAnchorCenter();

    const anchor = anchorSelector ? document.querySelector(anchorSelector) : null;
    const resizeObserver =
      anchor instanceof HTMLElement && typeof ResizeObserver !== "undefined"
        ? new ResizeObserver(() => updateAnchorCenter())
        : null;

    if (anchor instanceof HTMLElement && resizeObserver) {
      resizeObserver.observe(anchor);
    }

    window.addEventListener("resize", updateAnchorCenter);
    window.addEventListener("scroll", updateAnchorCenter, true);

    return () => {
      resizeObserver?.disconnect();
      window.removeEventListener("resize", updateAnchorCenter);
      window.removeEventListener("scroll", updateAnchorCenter, true);
    };
  }, [isOpen, anchorSelector]);

  // Click outside to close
  useEffect(() => {
    if (!isOpen) return;

    const handleMouseDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    document.addEventListener("mousedown", handleMouseDown);
    return () => document.removeEventListener("mousedown", handleMouseDown);
  }, [isOpen, onClose]);

  return {
    // State
    searchQuery,
    setSearchQuery,
    highlightedIndex,
    setHighlightedIndex,
    anchorCenterX,

    // Refs
    inputRef,
    containerRef,
    highlightedItemRef,

    // Store state
    activePlanId,
    planCandidates,
    isLoading,
    error,

    // Derived data
    filteredCandidates,
    canClearPlan,

    // Handlers
    handleKeyDown,
    handleSelect,
    handleClear,
    handleRetry,
  };
}
