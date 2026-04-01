/**
 * usePlanBrowserSearch - search state and debounce logic for PlanBrowser
 *
 * Manages the search input state with 300ms debounce, loading indicator
 * derivation, and expand state snapshot/restore for auto-expand behavior.
 *
 * Expand state snapshot timing:
 * - Captured ONCE on empty→non-empty transition of debouncedSearch
 * - Restored on non-empty→empty transition (when search is cleared)
 * - No re-capture on every keystroke
 */

import { useState, useRef, useEffect, useCallback } from "react";
import type { SessionGroup } from "@/components/Ideation/planBrowserUtils";

const DEBOUNCE_MS = 300;

export interface UsePlanBrowserSearchResult {
  searchTerm: string;
  debouncedSearch: string;
  isSearchActive: boolean;
  isSearchLoading: boolean;
  handleSearchChange: (value: string) => void;
  handleSearchClear: () => void;
}

export function usePlanBrowserSearch(
  groupOpen: Record<SessionGroup, boolean>,
  setGroupOpen: React.Dispatch<React.SetStateAction<Record<SessionGroup, boolean>>>
): UsePlanBrowserSearchResult {
  const [searchTerm, setSearchTerm] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const preSearchExpandStateRef = useRef<Record<SessionGroup, boolean> | null>(null);

  // Debounce: update debouncedSearch 300ms after last searchTerm change
  useEffect(() => {
    if (debounceTimerRef.current !== null) {
      clearTimeout(debounceTimerRef.current);
    }
    debounceTimerRef.current = setTimeout(() => {
      setDebouncedSearch(searchTerm);
    }, DEBOUNCE_MS);

    return () => {
      if (debounceTimerRef.current !== null) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, [searchTerm]);

  // Snapshot/restore expand state on debouncedSearch transitions
  const prevDebouncedRef = useRef(debouncedSearch);
  useEffect(() => {
    const prev = prevDebouncedRef.current;
    const curr = debouncedSearch;
    prevDebouncedRef.current = curr;

    if (prev === "" && curr !== "") {
      // empty→non-empty: capture pre-search expand state once
      preSearchExpandStateRef.current = { ...groupOpen };
    } else if (prev !== "" && curr === "") {
      // non-empty→empty: restore pre-search expand state
      if (preSearchExpandStateRef.current !== null) {
        setGroupOpen(preSearchExpandStateRef.current);
        preSearchExpandStateRef.current = null;
      }
    }
  }, [debouncedSearch, groupOpen, setGroupOpen]);

  const handleSearchChange = useCallback((value: string) => {
    setSearchTerm(value);
  }, []);

  const handleSearchClear = useCallback(() => {
    setSearchTerm("");
    // Immediately clear debounce timer and sync debouncedSearch to "" without waiting
    if (debounceTimerRef.current !== null) {
      clearTimeout(debounceTimerRef.current);
      debounceTimerRef.current = null;
    }
    setDebouncedSearch("");
  }, []);

  const isSearchActive = debouncedSearch !== "";
  // Loading: debounce pending (searchTerm differs from debouncedSearch) or debounced but not yet resolved
  const isSearchLoading = searchTerm !== debouncedSearch;

  return {
    searchTerm,
    debouncedSearch,
    isSearchActive,
    isSearchLoading,
    handleSearchChange,
    handleSearchClear,
  };
}
