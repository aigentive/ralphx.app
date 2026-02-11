/**
 * useChatAutoScroll - Unified scroll behavior for all chat components
 *
 * Provides smart bottom detection, auto-scroll control, streaming-aware triggers,
 * and manual scroll-to-bottom functionality. Ensures consistent scroll behavior
 * across ChatPanel, IntegratedChatPanel, and all Virtuoso-based message lists.
 *
 * Key features:
 * - Bottom tracking with 150px threshold
 * - Auto-scroll on message count increase OR streaming content changes
 * - Respects user manual scroll (pauses when scrolled up)
 * - Manual override via scrollToBottom()
 * - Disableable for history time-travel mode
 * - RAF debouncing for streaming updates to prevent scroll thrashing
 */

import { useRef, useCallback, useEffect, useState } from "react";

// ============================================================================
// Types
// ============================================================================

export interface UseChatAutoScrollProps {
  /** Number of messages - triggers scroll when count increases */
  messageCount: number;
  /** Is agent currently streaming */
  isStreaming: boolean;
  /** Hash of streaming content - triggers scroll when content changes (RAF-debounced) */
  streamingHash?: unknown;
  /** Disable auto-scroll (for history mode) */
  disabled?: boolean;
}

export interface UseChatAutoScrollReturn {
  /** Ref to attach to scroll container (for div-based components) */
  containerRef: React.RefObject<HTMLDivElement | null>;
  /** Ref to attach to end-of-messages marker (for div-based components) */
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  /** Is user at bottom of scroll area */
  isAtBottom: boolean;
  /** Should auto-scroll (computed: isAtBottom && !disabled) */
  shouldAutoScroll: boolean;
  /** Manual scroll-to-bottom function */
  scrollToBottom: () => void;
  /** Virtuoso callback: atBottomStateChange */
  handleAtBottomStateChange: (atBottom: boolean) => void;
  /** Virtuoso callback: followOutput */
  handleFollowOutput: (isAtBottom: boolean) => "smooth" | false;
}

// ============================================================================
// Hook
// ============================================================================

export function useChatAutoScroll({
  messageCount,
  isStreaming,
  streamingHash,
  disabled = false,
}: UseChatAutoScrollProps): UseChatAutoScrollReturn {
  // Refs for div-based scroll components
  const containerRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Track if user is at bottom
  const [isAtBottom, setIsAtBottom] = useState(true);

  // Computed: should we auto-scroll?
  const shouldAutoScroll = isAtBottom && !disabled;

  // Virtuoso callback: update bottom state when user scrolls
  const handleAtBottomStateChange = useCallback((atBottom: boolean) => {
    setIsAtBottom(atBottom);
  }, []);

  // Virtuoso callback: control followOutput behavior
  const handleFollowOutput = useCallback(
    (atBottom: boolean) => {
      if (atBottom && !disabled) return "smooth" as const;
      return false as const;
    },
    [disabled]
  );

  // Manual scroll-to-bottom
  const scrollToBottom = useCallback(() => {
    setIsAtBottom(true);
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, []);

  // Auto-scroll on message count increase
  useEffect(() => {
    if (shouldAutoScroll && messageCount > 0) {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [shouldAutoScroll, messageCount]);

  // Auto-scroll on streaming content changes (RAF-debounced)
  useEffect(() => {
    if (!shouldAutoScroll || !isStreaming || streamingHash === undefined) {
      return undefined;
    }

    // Use RAF to debounce scroll updates during streaming
    let rafId: number | null = null;
    rafId = requestAnimationFrame(() => {
      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    });

    return () => {
      if (rafId !== null) cancelAnimationFrame(rafId);
    };
  }, [shouldAutoScroll, isStreaming, streamingHash]);

  return {
    containerRef,
    messagesEndRef,
    isAtBottom,
    shouldAutoScroll,
    scrollToBottom,
    handleAtBottomStateChange,
    handleFollowOutput,
  };
}
