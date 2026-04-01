/**
 * useTeamKeyboardNav — Tmux-inspired prefix key navigation for team split view
 *
 * Ctrl+B activates prefix mode, then:
 *   ←/→ or h/l → switch coordinator/teammate column
 *   ↑/↓ or j/k → switch between teammate panes
 *   1-5 → jump to teammate by index
 *   z → toggle maximize focused pane
 *   - → minimize focused pane
 *   = → reset all pane sizes
 *   x → stop focused agent
 *   Escape → cancel prefix mode
 *
 * Auto-cancels prefix after 1.5 seconds.
 */

import { useEffect, useRef, useCallback } from "react";

import { stopTeammate } from "@/api/team";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import { useTeamStore, type TeammateState } from "@/stores/teamStore";

const PREFIX_TIMEOUT_MS = 1500;
const COORDINATOR_PANE = "coordinator";

export function useTeamKeyboardNav(enabled: boolean, contextKey: string | null) {
  const setFocusedPane = useSplitPaneStore((s) => s.setFocusedPane);
  const setPrefixKeyActive = useSplitPaneStore((s) => s.setPrefixKeyActive);
  const prefixTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isPrefixRef = useRef(false);

  const clearPrefixTimeout = useCallback(() => {
    if (prefixTimeoutRef.current !== null) {
      clearTimeout(prefixTimeoutRef.current);
      prefixTimeoutRef.current = null;
    }
  }, []);

  const deactivatePrefix = useCallback(() => {
    isPrefixRef.current = false;
    setPrefixKeyActive(false);
    clearPrefixTimeout();
  }, [setPrefixKeyActive, clearPrefixTimeout]);

  const activatePrefix = useCallback(() => {
    isPrefixRef.current = true;
    setPrefixKeyActive(true);
    clearPrefixTimeout();
    prefixTimeoutRef.current = setTimeout(deactivatePrefix, PREFIX_TIMEOUT_MS);
  }, [setPrefixKeyActive, clearPrefixTimeout, deactivatePrefix]);

  useEffect(() => {
    if (!enabled || !contextKey) return;

    const getTeammateNames = (): string[] => {
      const team = useTeamStore.getState().activeTeams[contextKey];
      if (!team) return [];
      return Object.values(team.teammates)
        .filter((t: TeammateState) => t.status !== "shutdown")
        .map((t: TeammateState) => t.name);
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      // Don't capture if user is typing in an input/textarea
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable
      ) {
        return;
      }

      // Phase 1: Activate prefix with Ctrl+B
      if (!isPrefixRef.current) {
        if (e.key === "b" && e.ctrlKey && !e.metaKey && !e.altKey) {
          e.preventDefault();
          activatePrefix();
        }
        return;
      }

      // Phase 2: Handle navigation key after prefix
      e.preventDefault();
      const focusedPane = useSplitPaneStore.getState().focusedPane;
      const teammates = getTeammateNames();

      switch (e.key) {
        // Left: focus coordinator
        case "ArrowLeft":
        case "h": {
          setFocusedPane(COORDINATOR_PANE);
          deactivatePrefix();
          break;
        }

        // Right: focus first teammate (or current if already on teammate side)
        case "ArrowRight":
        case "l": {
          if (teammates.length > 0) {
            const isOnTeammate = focusedPane !== null && focusedPane !== COORDINATOR_PANE;
            if (!isOnTeammate) {
              setFocusedPane(teammates[0]!);
            }
          }
          deactivatePrefix();
          break;
        }

        // Up: previous teammate in list
        case "ArrowUp":
        case "k": {
          if (teammates.length > 0) {
            const currentIdx = focusedPane ? teammates.indexOf(focusedPane) : -1;
            const nextIdx = currentIdx > 0 ? currentIdx - 1 : teammates.length - 1;
            setFocusedPane(teammates[nextIdx]!);
          }
          deactivatePrefix();
          break;
        }

        // Down: next teammate in list
        case "ArrowDown":
        case "j": {
          if (teammates.length > 0) {
            const currentIdx = focusedPane ? teammates.indexOf(focusedPane) : -1;
            const nextIdx = currentIdx < teammates.length - 1 ? currentIdx + 1 : 0;
            setFocusedPane(teammates[nextIdx]!);
          }
          deactivatePrefix();
          break;
        }

        // z: toggle maximize focused pane
        case "z": {
          if (focusedPane && focusedPane !== COORDINATOR_PANE) {
            const pane = useSplitPaneStore.getState().panes[focusedPane];
            if (pane?.maximized) {
              useSplitPaneStore.getState().restorePane(focusedPane);
            } else {
              useSplitPaneStore.getState().maximizePane(focusedPane);
            }
          }
          deactivatePrefix();
          break;
        }

        // -: minimize focused pane
        case "-": {
          if (focusedPane && focusedPane !== COORDINATOR_PANE) {
            useSplitPaneStore.getState().minimizePane(focusedPane);
          }
          deactivatePrefix();
          break;
        }

        // =: reset all pane sizes to equal
        case "=": {
          useSplitPaneStore.getState().resetPaneSizes();
          deactivatePrefix();
          break;
        }

        // x: stop focused agent
        case "x": {
          if (focusedPane && focusedPane !== COORDINATOR_PANE) {
            const team = useTeamStore.getState().activeTeams[contextKey];
            if (team) {
              void stopTeammate(team.teamName, focusedPane);
            }
          }
          deactivatePrefix();
          break;
        }

        // Escape: cancel prefix
        case "Escape": {
          deactivatePrefix();
          break;
        }

        // 1-5: jump to teammate by index
        default: {
          const num = parseInt(e.key, 10);
          if (num >= 1 && num <= 5) {
            const idx = num - 1;
            if (idx < teammates.length) {
              setFocusedPane(teammates[idx]!);
            }
          }
          deactivatePrefix();
          break;
        }
      }
    };

    document.addEventListener("keydown", handleKeyDown);

    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      deactivatePrefix();
    };
  }, [enabled, contextKey, activatePrefix, deactivatePrefix, setFocusedPane]);
}
