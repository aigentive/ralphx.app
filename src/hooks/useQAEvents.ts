/**
 * useQAEvents hook - Tauri event listeners for QA events
 *
 * Listens to QA-related events and updates the qaStore accordingly.
 * Handles qa:prep and qa:test events with runtime validation.
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { QAPrepEventSchema, QATestEventSchema } from "@/types/events";
import { useQAStore } from "@/stores/qaStore";

/**
 * Hook to listen for QA events from the backend
 *
 * Listens to 'qa:prep' and 'qa:test' events and updates the QA store.
 * Validates incoming events using Zod schemas before processing.
 *
 * @param taskId - Optional task ID to filter events for
 *
 * @example
 * ```tsx
 * function QAPanel({ taskId }: { taskId: string }) {
 *   useQAEvents(taskId);
 *   const taskQA = useQAStore((s) => s.taskQA[taskId]);
 *   return <QAStatusDisplay data={taskQA} />;
 * }
 * ```
 */
export function useQAEvents(taskId?: string) {
  const setLoadingTask = useQAStore((s) => s.setLoadingTask);
  const setError = useQAStore((s) => s.setError);

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    // Listen for QA prep events
    unlisteners.push(
      listen<unknown>("qa:prep", (event) => {
        const parsed = QAPrepEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error("Invalid QA prep event:", parsed.error.message);
          return;
        }

        const prepEvent = parsed.data;

        // Filter by taskId if provided
        if (taskId && prepEvent.taskId !== taskId) {
          return;
        }

        switch (prepEvent.type) {
          case "started":
            setLoadingTask(prepEvent.taskId, true);
            break;
          case "completed":
            setLoadingTask(prepEvent.taskId, false);
            break;
          case "failed":
            setLoadingTask(prepEvent.taskId, false);
            setError(
              `QA Prep failed for task ${prepEvent.taskId}: ${prepEvent.error ?? "Unknown error"}`
            );
            break;
        }
      })
    );

    // Listen for QA test events
    unlisteners.push(
      listen<unknown>("qa:test", (event) => {
        const parsed = QATestEventSchema.safeParse(event.payload);

        if (!parsed.success) {
          console.error("Invalid QA test event:", parsed.error.message);
          return;
        }

        const testEvent = parsed.data;

        // Filter by taskId if provided
        if (taskId && testEvent.taskId !== taskId) {
          return;
        }

        switch (testEvent.type) {
          case "started":
            setLoadingTask(testEvent.taskId, true);
            break;
          case "passed":
            setLoadingTask(testEvent.taskId, false);
            break;
          case "failed":
            setLoadingTask(testEvent.taskId, false);
            setError(
              `QA Tests failed for task ${testEvent.taskId}: ${testEvent.error ?? "Unknown error"}`
            );
            break;
        }
      })
    );

    return () => {
      unlisteners.forEach((unlisten) => unlisten.then((fn) => fn()));
    };
  }, [taskId, setLoadingTask, setError]);
}
