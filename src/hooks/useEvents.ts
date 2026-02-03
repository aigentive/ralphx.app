/**
 * Event hooks - Tauri event listeners with type-safe validation
 *
 * Provides hooks for listening to backend events (task changes, agent messages,
 * supervisor alerts) and updating local stores in response.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import type { AgentMessageEvent } from "@/types/events";
import { useActivityStore } from "@/stores/activityStore";

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
  const bus = useEventBus();
  const addMessage = useActivityStore((s) => s.addMessage);

  useEffect(() => {
    if (import.meta.env.DEV) {
      console.log("[useAgentEvents] Setting up listener for agent:message", {
        taskId,
      });
    }
    const unsubscribes = [
      bus.subscribe<AgentMessageEvent>("agent:message", (payload) => {
      if (import.meta.env.DEV) {
        console.log(
          "[useAgentEvents] Received event:",
          payload.type,
          payload.taskId
        );
      }
      if (!taskId || payload.taskId === taskId) {
        addMessage(payload);
      }
      }),
      bus.subscribe<{
        context_type: string;
        context_id: string;
        content: string;
      }>("agent:message_created", (payload) => {
        if (payload.context_type !== "task_execution") {
          return;
        }
        const message: AgentMessageEvent = {
          taskId: payload.context_id,
          type: "text",
          content: payload.content ?? "",
          timestamp: Date.now(),
        };
        if (!taskId || message.taskId === taskId) {
          addMessage(message);
        }
      }),
    ];

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, taskId, addMessage]);
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
  const bus = useEventBus();
  const addAlert = useActivityStore((s) => s.addAlert);

  useEffect(() => {
    return bus.subscribe<{
      taskId: string;
      severity: "low" | "medium" | "high" | "critical";
      type: "error" | "loop_detected" | "stuck" | "escalation";
      message: string;
    }>("supervisor:alert", (payload) => {
      addAlert(payload);
    });
  }, [bus, addAlert]);
}

/**
 * Hook to listen for file change events
 *
 * Listens to 'file:change' events for file system updates.
 * This is a placeholder for future implementation.
 */
export function useFileChangeEvents() {
  const bus = useEventBus();

  useEffect(() => {
    return bus.subscribe<unknown>("file:change", (_payload) => {
      // TODO: Implement file change handling
    });
  }, [bus]);
}

// Re-export specialized event hooks
export { useTaskEvents } from "./useEvents.task";
export { useReviewEvents } from "./useEvents.review";
export { useProposalEvents } from "./useEvents.proposal";
export { useExecutionErrorEvents } from "./useEvents.execution";
export { useStepEvents } from "./useStepEvents";
export { useRecoveryPromptEvents } from "./useEvents.recovery";
