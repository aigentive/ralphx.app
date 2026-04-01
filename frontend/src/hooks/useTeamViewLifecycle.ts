/**
 * useTeamViewLifecycle — Auto-switch to/from team view on team lifecycle events
 *
 * On team:created → saves current view, switches to "team" view
 * On team:disbanded → restores previous view
 *
 * Uses uiStore.setCurrentView and uiStore.setPreviousView for view management.
 */

import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { useUiStore } from "@/stores/uiStore";
import { useSplitPaneStore } from "@/stores/splitPaneStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import type { ContextType } from "@/types/chat-conversation";
import type { Unsubscribe } from "@/lib/event-bus";

export function useTeamViewLifecycle(contextKey: string | null) {
  const bus = useEventBus();
  const setCurrentView = useUiStore((s) => s.setCurrentView);
  const setPreviousView = useUiStore((s) => s.setPreviousView);
  const resetSplitPane = useSplitPaneStore((s) => s.reset);
  const setContextKey = useSplitPaneStore((s) => s.setContextKey);

  useEffect(() => {
    if (!contextKey) return;

    const unsubs: Unsubscribe[] = [];

    const matchKey = (payload: { context_type: string; context_id: string }): boolean => {
      const key = buildStoreKey(payload.context_type as ContextType, payload.context_id);
      return key === contextKey;
    };

    // team:created → save current view and switch to team view
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
        team_name: string;
      }>("team:created", (payload) => {
        if (!matchKey(payload)) return;

        const currentView = useUiStore.getState().currentView;
        if (currentView !== "team") {
          setPreviousView(currentView);
          setCurrentView("team");
        }
        setContextKey(contextKey);
      }),
    );

    // team:disbanded → restore previous view
    unsubs.push(
      bus.subscribe<{
        context_type: string;
        context_id: string;
      }>("team:disbanded", (payload) => {
        if (!matchKey(payload)) return;

        const { previousView, currentView } = useUiStore.getState();
        if (currentView === "team" && previousView) {
          setCurrentView(previousView);
          setPreviousView(null);
        }
        resetSplitPane();
      }),
    );

    return () => unsubs.forEach((u) => u());
  }, [bus, contextKey, setCurrentView, setPreviousView, setContextKey, resetSplitPane]);
}
