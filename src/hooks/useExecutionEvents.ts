/**
 * useExecutionEvents hook - Real-time execution status event listener
 *
 * Listens to execution events from the backend and updates the UI store
 * immediately to provide instant feedback for execution state changes.
 *
 * Events:
 * - execution:status_changed: Running count, pause state, etc.
 * - execution:queue_changed: Queued task count
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useUiStore } from "@/stores/uiStore";

/**
 * Event payload for execution:status_changed
 */
interface ExecutionStatusEvent {
  isPaused: boolean;
  runningCount: number;
  maxConcurrent: number;
  reason: string;
  timestamp: string;
}

/**
 * Event payload for execution:queue_changed
 */
interface ExecutionQueueEvent {
  queuedCount: number;
  timestamp: string;
}

/**
 * Hook to listen for execution events from the backend
 *
 * Listens to 'execution:status_changed' and 'execution:queue_changed' events
 * and updates the UI store immediately for real-time feedback.
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
  const setExecutionStatus = useUiStore((state) => state.setExecutionStatus);
  const setExecutionQueuedCount = useUiStore(
    (state) => state.setExecutionQueuedCount
  );

  useEffect(() => {
    // Listen for execution:status_changed events
    const unlistenStatus: Promise<UnlistenFn> = listen<ExecutionStatusEvent>(
      "execution:status_changed",
      (event) => {
        const { isPaused, runningCount, maxConcurrent } = event.payload;
        setExecutionStatus({
          isPaused,
          runningCount,
          maxConcurrent,
          // Preserve current queuedCount - will be updated by queue_changed event
          queuedCount: useUiStore.getState().executionStatus.queuedCount,
          canStartTask: !isPaused && runningCount < maxConcurrent,
        });
      }
    );

    // Listen for execution:queue_changed events
    const unlistenQueue: Promise<UnlistenFn> = listen<ExecutionQueueEvent>(
      "execution:queue_changed",
      (event) => {
        setExecutionQueuedCount(event.payload.queuedCount);
      }
    );

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenQueue.then((fn) => fn());
    };
  }, [setExecutionStatus, setExecutionQueuedCount]);
}
