/**
 * Event batching hooks for performance optimization
 *
 * High-frequency events (like agent messages during streaming) are buffered
 * and flushed periodically to prevent render thrashing.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect, useRef, useState } from "react";
import { useEventBus } from "@/providers/EventProvider";
import type { AgentMessageEvent } from "@/types/events";

/** Flush interval in milliseconds */
const FLUSH_INTERVAL_MS = 50;

/**
 * Hook to batch agent messages for performance
 *
 * Buffers incoming agent:message events and flushes them every 50ms
 * to prevent excessive re-renders during streaming.
 *
 * @param taskId - The task ID to filter messages for
 * @returns Array of accumulated messages
 *
 * @example
 * ```tsx
 * function TaskActivityStream({ taskId }: { taskId: string }) {
 *   const messages = useBatchedAgentMessages(taskId);
 *
 *   return (
 *     <div>
 *       {messages.map((msg, i) => (
 *         <MessageBubble key={i} message={msg} />
 *       ))}
 *     </div>
 *   );
 * }
 * ```
 */
export function useBatchedAgentMessages(taskId: string): AgentMessageEvent[] {
  const bus = useEventBus();
  const bufferRef = useRef<AgentMessageEvent[]>([]);
  const [messages, setMessages] = useState<AgentMessageEvent[]>([]);

  // Set up flush interval
  useEffect(() => {
    const interval = setInterval(() => {
      if (bufferRef.current.length > 0) {
        setMessages((prev) => [...prev, ...bufferRef.current]);
        bufferRef.current = [];
      }
    }, FLUSH_INTERVAL_MS);

    return () => clearInterval(interval);
  }, []);

  // Set up event listener
  useEffect(() => {
    return bus.subscribe<AgentMessageEvent>("agent:message", (payload) => {
      if (payload.taskId === taskId) {
        bufferRef.current.push(payload);
      }
    });
  }, [bus, taskId]);

  return messages;
}
