/**
 * useExecutionEvents hook - Real-time execution status event listener
 *
 * Listens to execution events from the backend and updates the UI store
 * immediately to provide instant feedback for execution state changes.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 *
 * Events:
 * - execution:status_changed: Running count, pause state, etc.
 * - execution:queue_changed: Queued task count
 *
 * Phase 82: Events now include projectId for per-project status updates.
 * Only updates UI store if the event is for the currently active project.
 */

import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { useUiStore } from "@/stores/uiStore";
import { useProjectStore } from "@/stores/projectStore";
import type { Unsubscribe } from "@/lib/event-bus";

/**
 * Event payload for execution:status_changed
 * Phase 82: Added projectId and globalMaxConcurrent
 */
interface ExecutionStatusEvent {
  isPaused: boolean;
  runningCount: number;
  maxConcurrent: number;
  globalMaxConcurrent?: number;
  projectId?: string;
  reason: string;
  timestamp: string;
}

/**
 * Event payload for execution:queue_changed
 * Phase 82: Added projectId
 */
interface ExecutionQueueEvent {
  queuedCount: number;
  projectId?: string;
  timestamp: string;
}

/**
 * Hook to listen for execution events from the backend
 *
 * Listens to 'execution:status_changed' and 'execution:queue_changed' events
 * and updates the UI store immediately for real-time feedback.
 *
 * Phase 82: Only updates UI store if the event's projectId matches the
 * currently active project (or if projectId is not specified in the event).
 *
 * This hook should be called once in the App component to enable
 * real-time execution status updates throughout the application.
 *
 * @example
 * ```tsx
 * function App() {
 *   useExecutionEvents(); // Enables real-time execution status
 *   return <AppContent />;
 * }
 * ```
 */
export function useExecutionEvents() {
  const bus = useEventBus();
  const setExecutionStatus = useUiStore((state) => state.setExecutionStatus);
  const setExecutionQueuedCount = useUiStore(
    (state) => state.setExecutionQueuedCount
  );

  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Listen for execution:status_changed events
    unsubscribes.push(
      bus.subscribe<ExecutionStatusEvent>("execution:status_changed", (payload) => {
        const { isPaused, runningCount, maxConcurrent, globalMaxConcurrent, projectId } = payload;

        // Phase 82: Only update if event is for the active project (or unscoped)
        const activeProjectId = useProjectStore.getState().activeProjectId;
        if (projectId && activeProjectId && projectId !== activeProjectId) {
          // Event is for a different project - ignore
          return;
        }

        setExecutionStatus({
          isPaused,
          runningCount,
          maxConcurrent,
          globalMaxConcurrent: globalMaxConcurrent ?? useUiStore.getState().executionStatus.globalMaxConcurrent,
          // Preserve current queuedCount - will be updated by queue_changed event
          queuedCount: useUiStore.getState().executionStatus.queuedCount,
          canStartTask: !isPaused && runningCount < maxConcurrent,
        });
      })
    );

    // Listen for execution:queue_changed events
    unsubscribes.push(
      bus.subscribe<ExecutionQueueEvent>("execution:queue_changed", (payload) => {
        const { queuedCount, projectId } = payload;

        // Phase 82: Only update if event is for the active project (or unscoped)
        const activeProjectId = useProjectStore.getState().activeProjectId;
        if (projectId && activeProjectId && projectId !== activeProjectId) {
          // Event is for a different project - ignore
          return;
        }

        setExecutionQueuedCount(queuedCount);
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, setExecutionStatus, setExecutionQueuedCount]);
}
