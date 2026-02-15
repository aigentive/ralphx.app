/**
 * usePaneEvents — Per-pane event subscription for auto-focus on activity
 *
 * Subscribes to agent:chunk and agent:run_started events filtered by teammate_name.
 * When a teammate becomes active (run_started), optionally auto-focuses that pane
 * in the splitPaneStore.
 *
 * Core streaming text routing is handled by useTeamEvents — this hook adds
 * pane-specific behavior (auto-focus) on top.
 */

import { useEffect, useRef } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import type { ContextType } from "@/types/chat-conversation";
import type { Unsubscribe } from "@/lib/event-bus";

interface UsePaneEventsOptions {
  /** Auto-focus pane when teammate becomes active */
  autoFocus?: boolean;
}

export function usePaneEvents(
  contextKey: string | null,
  teammateName: string | null,
  options: UsePaneEventsOptions = {},
) {
  const { autoFocus = false } = options;
  const bus = useEventBus();
  const setFocusedPane = useSplitPaneStore((s) => s.setFocusedPane);

  // Use ref for autoFocus to avoid re-subscribing on toggle
  const autoFocusRef = useRef(autoFocus);
  autoFocusRef.current = autoFocus;

  useEffect(() => {
    if (!contextKey || !teammateName) return;

    const unsubs: Unsubscribe[] = [];

    // Helper: check if event matches our context
    const matchKey = (payload: { context_type: string; context_id: string }): boolean => {
      const key = buildStoreKey(payload.context_type as ContextType, payload.context_id);
      return key === contextKey;
    };

    // Auto-focus pane when teammate starts running
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name?: string | null;
      }>("agent:run_started", (payload) => {
        if (
          autoFocusRef.current &&
          payload.teammate_name === teammateName &&
          matchKey(payload)
        ) {
          setFocusedPane(teammateName);
        }
      }),
    );

    // Auto-focus on first chunk if pane is unfocused
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        teammate_name?: string | null;
        text: string;
      }>("agent:chunk", (payload) => {
        if (
          autoFocusRef.current &&
          payload.teammate_name === teammateName &&
          matchKey(payload)
        ) {
          // Only auto-focus if no pane is currently focused
          const current = useSplitPaneStore.getState().focusedPane;
          if (current === null) {
            setFocusedPane(teammateName);
          }
        }
      }),
    );

    return () => unsubs.forEach((u) => u());
  }, [bus, contextKey, teammateName, setFocusedPane]);
}
