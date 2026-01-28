/**
 * Event hooks - Tauri event listeners with type-safe validation
 *
 * Provides hooks for listening to backend events (task changes, agent messages,
 * supervisor alerts) and updating local stores in response.
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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

// Re-export specialized event hooks
export { useTaskEvents } from "./useEvents.task";
export { useReviewEvents } from "./useEvents.review";
export { useProposalEvents } from "./useEvents.proposal";
export { useExecutionErrorEvents } from "./useEvents.execution";
export { useStepEvents } from "./useStepEvents";
