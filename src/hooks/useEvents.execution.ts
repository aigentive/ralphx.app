/**
 * Execution event hooks - Tauri execution error event listeners
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useEventBus } from "@/providers/EventProvider";
import { useChatStore } from "@/stores/chatStore";
import { taskKeys } from "@/hooks/useTasks";
import type { Unsubscribe } from "@/lib/event-bus";

/**
 * Hook to listen for execution error events
 *
 * Listens to 'execution:error' and 'execution:stderr' events from the backend.
 * When an error occurs:
 * - Resets the agent running state to unstick the UI
 * - Shows a toast notification with the error message
 * - Logs detailed stderr for debugging
 *
 * @example
 * ```tsx
 * function App() {
 *   useExecutionErrorEvents(); // Auto-handles execution errors
 *   return <TaskBoard />;
 * }
 * ```
 */
export function useExecutionErrorEvents() {
  const bus = useEventBus();
  const setAgentRunning = useChatStore((s) => s.setAgentRunning);
  const queryClient = useQueryClient();

  useEffect(() => {
    const unsubscribes: Unsubscribe[] = [];

    // Listen for execution errors
    unsubscribes.push(
      bus.subscribe<{
        conversation_id?: string;
        task_id?: string;
        error: string;
        stderr?: string;
      }>("execution:error", (payload) => {
        console.error("Execution error received:", payload);

        // Reset agent running state to unstick the UI
        // Use task context key if task_id is present
        if (payload.task_id) {
          setAgentRunning(`task:${payload.task_id}`, false);
        }

        // Invalidate queries so UI refreshes
        queryClient.invalidateQueries({ queryKey: ["chat"] });
        queryClient.invalidateQueries({ queryKey: taskKeys.lists() });

        // Show toast notification
        toast.error("Agent execution failed", {
          description: payload.error.slice(0, 200),
          duration: 10000, // Keep error visible longer
        });
      })
    );

    // Listen for stderr events (useful for debugging)
    unsubscribes.push(
      bus.subscribe<{
        conversation_id: string;
        task_id?: string;
        content: string;
      }>("execution:stderr", (payload) => {
        // Log stderr for debugging but don't show toast for every line
        console.warn("[Agent STDERR]", payload.content);
      })
    );

    return () => {
      unsubscribes.forEach((unsub) => unsub());
    };
  }, [bus, setAgentRunning, queryClient]);
}
