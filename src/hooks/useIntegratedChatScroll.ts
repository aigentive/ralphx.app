/**
 * useIntegratedChatScroll - Auto-scroll logic for IntegratedChatPanel
 *
 * Handles:
 * - Auto-scroll to bottom when messages change
 * - Instant scroll when panel expands
 * - Auto-scroll during streaming (with RAF debouncing)
 */

import { useEffect, useRef } from "react";

interface UseIntegratedChatScrollProps {
  messagesData: unknown[];
  chatCollapsed: boolean;
  isAgentRunning: boolean;
  streamingToolCallsLength: number;
}

export function useIntegratedChatScroll({
  messagesData,
  chatCollapsed,
  isAgentRunning,
  streamingToolCallsLength,
}: UseIntegratedChatScrollProps) {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const scrollRAFRef = useRef<number | null>(null);

  // Auto-scroll to bottom when messages change
  useEffect(() => {
    if (messagesEndRef.current && messagesData.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messagesData.length]);

  // Scroll to bottom instantly when panel expands
  useEffect(() => {
    if (!chatCollapsed && messagesEndRef.current && messagesData.length) {
      messagesEndRef.current.scrollIntoView({ behavior: "instant" });
    }
  }, [chatCollapsed, messagesData.length]);

  // Auto-scroll during streaming (tool calls and agent running)
  // Use requestAnimationFrame to debounce rapid updates
  useEffect(() => {
    if (isAgentRunning && messagesEndRef.current) {
      // Cancel any pending scroll
      if (scrollRAFRef.current) {
        cancelAnimationFrame(scrollRAFRef.current);
      }
      // Schedule scroll on next frame
      scrollRAFRef.current = requestAnimationFrame(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
        scrollRAFRef.current = null;
      });
    }
    return () => {
      if (scrollRAFRef.current) {
        cancelAnimationFrame(scrollRAFRef.current);
      }
    };
  }, [isAgentRunning, streamingToolCallsLength]);

  return { messagesEndRef };
}
