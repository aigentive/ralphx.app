/**
 * Event hooks - Tauri event listeners with type-safe validation
 *
 * Provides hooks for listening to backend events (task changes, agent messages,
 * supervisor alerts) and updating local stores in response.
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import {
  TaskEventSchema,
  ReviewEventSchema,
  type AgentMessageEvent,
} from "@/types/events";
import { useTaskStore } from "@/stores/taskStore";
import { useActivityStore } from "@/stores/activityStore";
import { reviewKeys } from "@/hooks/useReviews";
import type { Task } from "@/types/task";

/**
 * Hook to listen for task events from the backend
 *
 * Listens to 'task:event' events and updates the task store accordingly.
 * Validates incoming events using TaskEventSchema before processing.
 *
 * @example
 * ```tsx
 * function App() {
 *   useTaskEvents(); // Sets up listener automatically
 *   return <TaskBoard />;
 * }
 * ```
 */
export function useTaskEvents() {
  const addTask = useTaskStore((s) => s.addTask);
  const updateTask = useTaskStore((s) => s.updateTask);
  const removeTask = useTaskStore((s) => s.removeTask);

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("task:event", (event) => {
      // Runtime validation of backend events
      const parsed = TaskEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid task event:", parsed.error.message);
        return;
      }

      const taskEvent = parsed.data;

      switch (taskEvent.type) {
        case "created":
          addTask(taskEvent.task);
          break;
        case "updated":
          // Cast to Partial<Task> for exactOptionalPropertyTypes compatibility
          updateTask(taskEvent.taskId, taskEvent.changes as Partial<Task>);
          break;
        case "deleted":
          removeTask(taskEvent.taskId);
          break;
        case "status_changed":
          updateTask(taskEvent.taskId, { internalStatus: taskEvent.to });
          break;
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addTask, updateTask, removeTask]);
}

/**
 * Hook to listen for agent message events
 *
 * Listens to 'agent:message' events and adds them to the activity store.
 * Can optionally filter by taskId.
 *
 * @param taskId - Optional task ID to filter messages for
 *
 * @example
 * ```tsx
 * function TaskActivityStream({ taskId }: { taskId: string }) {
 *   useAgentEvents(taskId);
 *   const messages = useActivityStore((s) => s.getMessagesForTask(taskId));
 *   return <MessageList messages={messages} />;
 * }
 * ```
 */
export function useAgentEvents(taskId?: string) {
  const addMessage = useActivityStore((s) => s.addMessage);

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<AgentMessageEvent>("agent:message", (event) => {
      if (!taskId || event.payload.taskId === taskId) {
        addMessage(event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [taskId, addMessage]);
}

/**
 * Hook to listen for supervisor alert events
 *
 * Listens to 'supervisor:alert' events and adds them to the activity store.
 *
 * @example
 * ```tsx
 * function SupervisorPanel() {
 *   useSupervisorAlerts();
 *   const alerts = useActivityStore((s) => s.alerts);
 *   return <AlertList alerts={alerts} />;
 * }
 * ```
 */
export function useSupervisorAlerts() {
  const addAlert = useActivityStore((s) => s.addAlert);

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<{
      taskId: string;
      severity: "low" | "medium" | "high" | "critical";
      type: "error" | "loop_detected" | "stuck" | "escalation";
      message: string;
    }>("supervisor:alert", (event) => {
      addAlert(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [addAlert]);
}

/**
 * Hook to listen for review events
 *
 * Listens to 'review:update' events and invalidates TanStack Query caches
 * to trigger refetching of review-related data.
 *
 * @example
 * ```tsx
 * function ReviewsPanel() {
 *   useReviewEvents(); // Auto-refreshes review data on backend events
 *   const { data } = usePendingReviews(projectId);
 *   return <ReviewList reviews={data} />;
 * }
 * ```
 */
export function useReviewEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("review:update", (event) => {
      // Runtime validation of backend events
      const parsed = ReviewEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid review event:", parsed.error.message);
        return;
      }

      const reviewEvent = parsed.data;

      // Always invalidate pending reviews (all events affect this)
      queryClient.invalidateQueries({
        queryKey: reviewKeys.pending(),
      });

      // For completed events, also invalidate task-specific queries
      if (reviewEvent.type === "completed") {
        queryClient.invalidateQueries({
          queryKey: reviewKeys.byTaskId(reviewEvent.taskId),
        });
        queryClient.invalidateQueries({
          queryKey: reviewKeys.stateHistoryById(reviewEvent.taskId),
        });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [queryClient]);
}

/**
 * Hook to listen for file change events
 *
 * Listens to 'file:change' events for file system updates.
 * This is a placeholder for future implementation.
 */
export function useFileChangeEvents() {
  useEffect(() => {
    const unlisten: Promise<UnlistenFn> = listen<unknown>("file:change", (_event) => {
      // TODO: Implement file change handling
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
