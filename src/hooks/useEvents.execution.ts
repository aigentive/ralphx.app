/**
 * Execution event hooks - Tauri execution error event listeners
 */

import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useChatStore } from "@/stores/chatStore";
import { taskKeys } from "@/hooks/useTasks";

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
  const setAgentRunning = useChatStore((s) => s.setAgentRunning);
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    // Listen for execution errors
    unlisteners.push(
      listen<{
        conversation_id?: string;
        task_id?: string;
        error: string;
        stderr?: string;
      }>("execution:error", (event) => {
        console.error("Execution error received:", event.payload);

        // Reset agent running state to unstick the UI
        // Use task context key if task_id is present
        if (event.payload.task_id) {
          setAgentRunning(`task:${event.payload.task_id}`, false);
        }

        // Invalidate queries so UI refreshes
        queryClient.invalidateQueries({ queryKey: ["chat"] });
        queryClient.invalidateQueries({ queryKey: taskKeys.lists() });

        // Show toast notification
        toast.error("Agent execution failed", {
          description: event.payload.error.slice(0, 200),
          duration: 10000, // Keep error visible longer
        });
      })
    );

    // Listen for stderr events (useful for debugging)
    unlisteners.push(
      listen<{
        conversation_id: string;
        task_id?: string;
        content: string;
      }>("execution:stderr", (event) => {
        // Log stderr for debugging but don't show toast for every line
        console.warn("[Agent STDERR]", event.payload.content);
      })
    );

    return () => {
      unlisteners.forEach((unlisten) => unlisten.then((fn) => fn()));
    };
  }, [setAgentRunning, queryClient]);
}
