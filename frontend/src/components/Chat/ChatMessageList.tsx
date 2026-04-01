/**
 * ChatMessageList - Virtualized message list for chat panels
 *
 * Wraps react-virtuoso with chat-specific rendering:
 * - Auto-scroll to bottom
 * - Failed run banner header
 * - Worker executing indicator
 * - Streaming tool calls / typing indicator footer
 */

import React, { forwardRef, useCallback, useEffect, useMemo, useRef, useState, useImperativeHandle } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { MessageItem } from "./MessageItem";
import { HookEventMessage } from "./HookEventMessage";
import { AutoVerificationCard } from "./AutoVerificationCard";
import { AUTO_VERIFICATION_KEY } from "@/types/ideation";
import {
  TypingIndicator,
  FailedRunBanner,
} from "./IntegratedChatPanel.components";
import { ToolCallIndicator } from "./ToolCallIndicator";
import type { ToolCall } from "./ToolCallIndicator";
import type { StreamingTask, StreamingContentBlock } from "@/types/streaming-task";
import type { ContentBlockItem } from "./MessageItem";
import type { HookEvent, HookStartedEvent } from "@/types/hook-event";
import { isDiffToolCall } from "./DiffToolCallView.utils";
import { DiffToolCallView } from "./DiffToolCallView";
import { TaskSubagentCard } from "./TaskSubagentCard";
import { useChatAutoScroll } from "@/hooks/useChatAutoScroll";
import { shouldUseWebkitSafeScrollBehavior } from "@/lib/platform-quirks";
import { useMessageAttachments } from "@/hooks/useMessageAttachments";
import { ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { MessageAttachment } from "./MessageAttachments";
import { useTeamStore, selectTeammateByName, selectTeamMessages, EMPTY_TEAM_MESSAGES } from "@/stores/teamStore";
import { ToolCallStoreKeyContext } from "./tool-widgets/ToolCallStoreKeyContext";
import type { TeamMessage } from "@/stores/teamStore";
import { TeamMessageBubble } from "./TeamMessageBubble";

// ============================================================================
// Constants
// ============================================================================

/** Delay for markdown content to render and expand before scroll correction */
const MARKDOWN_RENDER_DELAY_MS = 300;

/** Shared bottom-detection threshold — used by both Virtuoso atBottomThreshold prop and rAF DOM reconciliation.
 *  Must match exactly so both agree on what "at bottom" means. */
export const AT_BOTTOM_THRESHOLD = 150;

/** Bucket size for text length change detection during streaming.
 *  ~2 visible lines per trigger (average line ~80 chars at standard chat width → 2 lines × 80 = 160, rounded to 150). */
export const TEXT_LENGTH_BUCKET_SIZE = 150;

/** Shared styles for content containers to handle long text */
const contentContainerStyle: React.CSSProperties = {
  maxWidth: "100%",
  overflowWrap: "break-word",
  wordBreak: "break-word",
};

/** Stable empty arrays — avoids new refs on each render when props are omitted */
const EMPTY_HOOK_EVENTS: HookEvent[] = [];
const EMPTY_ACTIVE_HOOKS: HookStartedEvent[] = [];

// ============================================================================
// Types
// ============================================================================

export interface ChatMessageData {
  id: string;
  role: string;
  content: string;
  createdAt: string;
  toolCalls?: ToolCall[] | null;
  contentBlocks?: ContentBlockItem[] | null;
  attachments?: MessageAttachment[];
  sender?: string | null;
  metadata?: string | null;
}

/** Discriminated union for timeline items when hook events are interleaved */
type TimelineItem =
  | { kind: "message"; data: ChatMessageData; sortTime: number }
  | { kind: "hook"; data: HookEvent | HookStartedEvent; sortTime: number }
  | { kind: "team_event"; data: TeamMessage; sortTime: number };

interface ChatMessageListProps {
  messages: ChatMessageData[];
  /** Conversation ID - used as key to force remount on conversation switch */
  conversationId: string | null;
  /** Show failed run banner */
  failedRun?: { id: string; errorMessage: string } | null;
  /** Callback when failed run banner is dismissed */
  onDismissFailedRun?: (runId: string) => void;
  /** Is agent currently sending/responding */
  isSending: boolean;
  isAgentRunning: boolean;
  /** Streaming tool calls to display */
  streamingToolCalls: ToolCall[];
  /** Streaming subagent tasks — Map keyed by tool_use_id */
  streamingTasks?: Map<string, StreamingTask>;
  /** Streaming content blocks (text and tool calls interleaved) */
  streamingContentBlocks?: StreamingContentBlock[];
  /** Optional timestamp to scroll to (for history mode) - scrolls to first message at or after this time */
  scrollToTimestamp?: string | null;
  /** Resolved hook events (completed + blocks) — optional, interleaved chronologically */
  hookEvents?: HookEvent[];
  /** Currently running hooks — optional, interleaved chronologically */
  activeHooks?: HookStartedEvent[];
  /** Whether the conversation is finalizing (between message_created and query refetch) */
  isFinalizing?: boolean;
  /** Team filter for message filtering (team mode) */
  teamFilter?: "lead" | string | undefined;
  /** Context key for team store lookup (team mode) */
  contextKey?: string | undefined;
}

// ============================================================================
// Component
// ============================================================================

export const ChatMessageList = forwardRef<VirtuosoHandle, ChatMessageListProps>(
  function ChatMessageList(
    {
      messages,
      conversationId,
      failedRun,
      onDismissFailedRun,
      isSending,
      isAgentRunning,
      streamingToolCalls,
      streamingTasks,
      streamingContentBlocks,
      scrollToTimestamp,
      hookEvents = EMPTY_HOOK_EVENTS,
      activeHooks = EMPTY_ACTIVE_HOOKS,
      isFinalizing = false,
      teamFilter,
      contextKey,
    },
    ref
  ) {
    const preferredScrollBehavior = shouldUseWebkitSafeScrollBehavior()
      ? "auto"
      : "smooth";

    // Internal ref for scroll operations
    const virtuosoRef = useRef<VirtuosoHandle>(null);
    const hasScrolledRef = useRef<string | null>(null);
    // Track previous shouldFilterLastAssistant to detect false→true→false transition
    const prevShouldFilterRef = useRef(false);
    // rAF reconciliation refs — used to keep isAtBottom accurate when footer grows
    const scrollerElRef = useRef<HTMLElement | null>(null);
    const reconcileRafRef = useRef<number | null>(null);
    const isTestEnv = import.meta.env.VITEST;

    // Footer ResizeObserver refs — for height-driven auto-scroll (G2 fix)
    const footerElRef = useRef<HTMLDivElement | null>(null);
    const footerResizeRafRef = useRef<number | null>(null);
    const footerObserverRef = useRef<ResizeObserver | null>(null);
    const footerPrevHeightRef = useRef<number>(-1); // -1 = uninitialized sentinel
    const footerMountedRef = useRef(false); // H2 fix: skip initial mount observation
    const hasFooterStreamingContentRef = useRef(false);

    // Forward the ref to parent
    useImperativeHandle(ref, () => virtuosoRef.current!, []);

    // Team system messages for inline display
    const teamMsgSelector = useMemo(
      () => contextKey ? selectTeamMessages(contextKey) : () => EMPTY_TEAM_MESSAGES,
      [contextKey],
    );
    const teamMessages = useTeamStore(teamMsgSelector);

    // Fetch attachments for all messages
    const { data: attachmentsMap } = useMessageAttachments(messages, conversationId);

    // Footer content hash — drives the streaming auto-scroll useEffect below.
    // NOTE: Virtuoso's followOutput does NOT react to context/Footer changes,
    // only to totalCount changes. We use autoscrollToBottom() imperatively instead.
    const totalChildCalls = useMemo(() => {
      if (!streamingTasks || streamingTasks.size === 0) return 0;
      let count = 0;
      for (const task of streamingTasks.values()) {
        count += task.childToolCalls.length;
      }
      return count;
    }, [streamingTasks]);

    // Tracks running max of text length across all streaming blocks.
    // State (not a ref) so changes propagate to footerContentHash and trigger autoscroll.
    // Math.max(prev, total) ensures the bucket never decreases mid-stream — prevents
    // bucket regression when tool_use blocks are inserted between text blocks.
    const [cumulativeTextLength, setCumulativeTextLength] = useState(0);

    // Recompute cumulative text length whenever streaming blocks change.
    // Resets to 0 when streaming ends (no blocks) so the next stream starts fresh.
    useEffect(() => {
      if (!streamingContentBlocks?.length) {
        setCumulativeTextLength(0);
        return;
      }
      const total = streamingContentBlocks.reduce(
        (sum, block) => block.type === "text" ? sum + block.text.length : sum, 0
      );
      setCumulativeTextLength(prev => Math.max(prev, total));
    }, [streamingContentBlocks]);

    const hasFooterStreamingContent =
      streamingToolCalls.length > 0 ||
      totalChildCalls > 0 ||
      (streamingTasks?.size ?? 0) > 0 ||
      (streamingContentBlocks?.length ?? 0) > 0;

    useEffect(() => {
      hasFooterStreamingContentRef.current = hasFooterStreamingContent;
    }, [hasFooterStreamingContent]);

    const footerContentHash = useMemo(() => ({
      toolCallCount: streamingToolCalls.length,
      // G1 fix: results update existing blocks (count unchanged) — track result arrivals separately
      toolResultCount: streamingToolCalls.filter(tc => tc.result != null || tc.error != null).length,
      childCallCount: totalChildCalls,
      taskCount: streamingTasks?.size ?? 0,
      contentBlockCount: streamingContentBlocks?.length ?? 0,
      textLengthBucket: Math.floor(cumulativeTextLength / TEXT_LENGTH_BUCKET_SIZE),
    }), [streamingToolCalls, totalChildCalls, streamingTasks?.size, streamingContentBlocks?.length, cumulativeTextLength]);

    // Streaming auto-scroll — followOutput only fires on totalCount changes,
    // NOT on Footer height growth. Call autoscrollToBottom() imperatively when
    // footer content changes to keep the view pinned during streaming.
    useEffect(() => {
      // Only react while the streaming footer actually has live content.
      // When finalization clears footer state, followOutput/query refresh handle
      // the message swap; forcing another footer scroll here creates overlap.
      if (scrollToTimestamp || !hasFooterStreamingContent) return;
      virtuosoRef.current?.autoscrollToBottom();
    }, [footerContentHash, hasFooterStreamingContent, scrollToTimestamp]);

    // Unified auto-scroll hook — Virtuoso followOutput handles new-message scroll,
    // while the useEffect above handles streaming footer growth.
    const {
      messagesEndRef,
      isAtBottom,
      isAtBottomRef,
      scrollToBottom,
      handleAtBottomStateChange,
      handleFollowOutput,
    } = useChatAutoScroll({
      messageCount: messages.length,
      disabled: !!scrollToTimestamp, // Disable auto-scroll in history mode
      virtuosoRef, // Route scrollToBottom through Virtuoso scrollToIndex
      conversationId, // Reset isAtBottom when conversation changes
    });

    // Keep scrollToTimestamp accessible via ref (avoids stale closure in ResizeObserver callback)
    const scrollToTimestampRef = useRef(scrollToTimestamp);
    useEffect(() => {
      scrollToTimestampRef.current = scrollToTimestamp;
    }, [scrollToTimestamp]);

    // rAF-throttled DOM reconciliation — keeps isAtBottom accurate when Virtuoso doesn't detect footer growth.
    // Runs outside React render cycle (DOM event handler, not useEffect) — no render loop risk.
    // rAF fires post-paint, so scrollHeight reads don't force layout recalc during React commit phase.
    const handleScrollReconcile = useCallback(() => {
      if (reconcileRafRef.current) return; // Already scheduled — skip
      reconcileRafRef.current = requestAnimationFrame(() => {
        reconcileRafRef.current = null;
        const el = scrollerElRef.current;
        if (!el) return;
        const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < AT_BOTTOM_THRESHOLD;
        // Only reconcile if state disagrees — avoids unnecessary setState
        if (atBottom !== isAtBottomRef.current) {
          handleAtBottomStateChange(atBottom);
        }
      });
    }, [handleAtBottomStateChange, isAtBottomRef]);

    // Attach passive scroll listener to Virtuoso's scroller element.
    // Passed to Virtuoso's scrollerRef prop so we capture the actual scroll container.
    const handleScrollerRef = useCallback((el: Window | HTMLElement | null) => {
      if (!(el instanceof HTMLElement)) {
        if (scrollerElRef.current) {
          scrollerElRef.current.removeEventListener("scroll", handleScrollReconcile);
          scrollerElRef.current = null;
        }
        return;
      }
      if (scrollerElRef.current && scrollerElRef.current !== el) {
        scrollerElRef.current.removeEventListener("scroll", handleScrollReconcile);
      }
      scrollerElRef.current = el;
      el.addEventListener("scroll", handleScrollReconcile, { passive: true });
    }, [handleScrollReconcile]);

    // Cleanup rAF and scroll listener on unmount
    useEffect(() => {
      return () => {
        if (reconcileRafRef.current) cancelAnimationFrame(reconcileRafRef.current);
        scrollerElRef.current?.removeEventListener("scroll", handleScrollReconcile);
      };
    }, [handleScrollReconcile]);

    // Stable callback ref for Footer element — creates ResizeObserver that detects footer height
    // changes (G2 fix: card expansion during streaming). Empty deps ensures observer is never
    // torn down due to prop changes.
    //
    // H1 analysis: Late tool results (after turn_completed) update finalized messages in the
    // timeline, not the footer. Virtuoso's followOutput handles timeline height changes natively.
    // The footer ResizeObserver only needs to cover the active streaming window, not post-stream updates.
    const handleFooterRef = useCallback((el: HTMLDivElement | null) => {
      // Cleanup old observer
      if (footerObserverRef.current) {
        footerObserverRef.current.disconnect();
        footerObserverRef.current = null;
      }
      footerElRef.current = el;
      footerMountedRef.current = false; // Reset mount flag on new element
      if (!el) return;

      footerObserverRef.current = new ResizeObserver((entries) => {
        const newHeight = entries[0]?.contentRect.height ?? 0;

        // H2 fix: Skip the very first observation after mount.
        // The first observation captures baseline height without triggering scroll.
        // Prevents jarring scroll jump when switching chat tabs or loading history.
        if (!footerMountedRef.current) {
          footerMountedRef.current = true;
          footerPrevHeightRef.current = newHeight;
          return;
        }

        // Only react to height increases, not width changes or shrinking
        if (newHeight <= footerPrevHeightRef.current) {
          footerPrevHeightRef.current = newHeight;
          return;
        }
        footerPrevHeightRef.current = newHeight;

        // M1 fix: Cancel-reschedule rAF — don't skip if pending.
        // Rapid sequential resizes each get a scroll attempt; the last one wins.
        if (footerResizeRafRef.current) {
          cancelAnimationFrame(footerResizeRafRef.current);
        }
        footerResizeRafRef.current = requestAnimationFrame(() => {
          footerResizeRafRef.current = null;
          // Read from refs — always current, no stale closure
          if (
            hasFooterStreamingContentRef.current &&
            isAtBottomRef.current &&
            !scrollToTimestampRef.current
          ) {
            virtuosoRef.current?.autoscrollToBottom();
          }
        });
      });
      footerObserverRef.current.observe(el);
    }, [isAtBottomRef]); // isAtBottomRef is a stable ref — included to satisfy exhaustive-deps without changing behavior

    // Cleanup Footer ResizeObserver and rAF on unmount
    useEffect(() => {
      return () => {
        footerObserverRef.current?.disconnect();
        footerObserverRef.current = null;
        if (footerResizeRafRef.current) {
          cancelAnimationFrame(footerResizeRafRef.current);
          footerResizeRafRef.current = null;
        }
      };
    }, []);

    // Scroll to specific timestamp for history mode (time-travel feature)
    // Finds the first message at or after the given timestamp and scrolls to it
    useEffect(() => {
      if (!scrollToTimestamp || messages.length === 0) return;

      const targetTime = new Date(scrollToTimestamp).getTime();
      const targetIndex = messages.findIndex(
        (msg) => new Date(msg.createdAt).getTime() >= targetTime
      );

      if (targetIndex >= 0) {
        // Add a small delay to ensure Virtuoso is ready
        const timeoutId = setTimeout(() => {
          virtuosoRef.current?.scrollToIndex({
            index: targetIndex,
            align: "start",
            behavior: preferredScrollBehavior,
          });
        }, MARKDOWN_RENDER_DELAY_MS);
        return () => clearTimeout(timeoutId);
      }
      return undefined;
    }, [scrollToTimestamp, messages, preferredScrollBehavior]);

    // Build timeline data for Virtuoso. Always wraps messages as TimelineItem
    // for consistent typing. When hook events exist, they're interleaved and sorted.
    const hasHookEvents = hookEvents.length > 0 || activeHooks.length > 0;

    // Filter logic: during active streaming OR when conversation is finalizing (between
    // message_created clearing state and query refetch completing), exclude the last
    // assistant message from DB to prevent duplication with streamingContentBlocks.
    //
    // isFinalizing is set to true (in the same React batch as clearing streaming state)
    // by useChatEvents on agent:message_created, and reset to false after 500ms. This
    // keeps the filter active through the timing window where streaming state is cleared
    // but the query refetch hasn't completed yet.
    //
    // Additionally, when isAgentRunning but no streaming content exists yet (the window
    // between DB empty-message creation and the first streaming event), filter the last
    // assistant message if its content is empty/whitespace — prevents the empty "pill" flash.
    const hasActiveStreaming = (streamingContentBlocks && streamingContentBlocks.length > 0) ||
                              (streamingTasks && streamingTasks.size > 0);
    const shouldFilterLastAssistant = hasActiveStreaming || isFinalizing;

    // When filter clears (streaming/finalizing ends), scroll to bottom so the newly
    // revealed finalized assistant message is visible.
    useEffect(() => {
      if (scrollToTimestamp) return; // Don't auto-scroll in history mode
      if (prevShouldFilterRef.current && !shouldFilterLastAssistant) {
        scrollToBottom();
      }
      prevShouldFilterRef.current = shouldFilterLastAssistant;
    }, [shouldFilterLastAssistant, scrollToBottom, scrollToTimestamp]);

    const timeline = useMemo((): TimelineItem[] => {
      const items: TimelineItem[] = [];

      // Exclude the streaming assistant message from DB when active streaming/finalizing —
      // it's being rendered live in streamingContentBlocks. Do NOT filter based solely on
      // isAgentRunning: during team sessions the lead runs for extended periods, and filtering
      // without active streaming blocks hides historical assistant messages between turns.
      //
      // Use ID-based filtering: find the assistant message with the most recent createdAt
      // (with id as tiebreaker) so filtering is stable regardless of array order.
      const filteredMessages = shouldFilterLastAssistant
        ? (() => {
            // Find the most recently created assistant message by timestamp (stable, not index)
            let latestAssistantId: string | null = null;
            let latestAssistantTime = -Infinity;
            for (const msg of messages) {
              if (msg.role === "assistant") {
                const t = new Date(msg.createdAt).getTime();
                if (t > latestAssistantTime || (t === latestAssistantTime && msg.id > (latestAssistantId ?? ""))) {
                  latestAssistantTime = t;
                  latestAssistantId = msg.id;
                }
              }
            }
            if (latestAssistantId !== null) {
              return messages.filter((msg) => msg.id !== latestAssistantId);
            }
            return messages;
          })()
        : messages;

      // Team filter: each tab (lead/teammate) loads its own conversation's messages via
      // useConversation, so all messages in the data set belong to that conversation.
      // No per-message filtering needed — the conversation switch handles the scoping.
      const teamFilteredMessages = filteredMessages;

      for (const msg of teamFilteredMessages) {
        // Enrich message with attachments if available
        const attachments = attachmentsMap?.get(msg.id);
        const enrichedMsg = attachments
          ? { ...msg, attachments }
          : msg;

        items.push({
          kind: "message",
          data: enrichedMsg,
          sortTime: new Date(msg.createdAt).getTime(),
        });
      }

      if (hasHookEvents) {
        for (const ev of hookEvents) {
          items.push({ kind: "hook", data: ev, sortTime: ev.timestamp });
        }
        for (const ev of activeHooks) {
          items.push({ kind: "hook", data: ev, sortTime: ev.timestamp });
        }
      }

      // Interleave team system messages (filtered by teammate tab)
      if (teamMessages.length > 0) {
        const filteredTeamMsgs = teamFilter
          ? teamMessages.filter((msg) => {
              if (teamFilter === "lead") {
                // Lead sees ALL team messages (lead is the orchestrator)
                return true;
              }
              return msg.from === teamFilter || msg.to === teamFilter || msg.to === "*";
            })
          : teamMessages;

        for (const msg of filteredTeamMsgs) {
          items.push({
            kind: "team_event",
            data: msg,
            sortTime: new Date(msg.timestamp).getTime(),
          });
        }
      }

      // Sort if we interleaved any non-message items
      if (hasHookEvents || teamMessages.length > 0) {
        items.sort((a, b) => a.sortTime - b.sortTime);
      }

      return items;
    }, [messages, hookEvents, activeHooks, hasHookEvents, shouldFilterLastAssistant, attachmentsMap, teamFilter, teamMessages]);

    // Initial load scroll — fires when conversation changes and timeline populates.
    // Uses one-shot ResizeObserver on the scroller element to detect when virtual
    // content has actually rendered, rather than a fixed-duration setTimeout guess.
    // Falls back to MARKDOWN_RENDER_DELAY_MS if scrollerElRef not yet available.
    useEffect(() => {
      if (!conversationId || timeline.length === 0 || hasScrolledRef.current === conversationId) return;

      const targetConversationId = conversationId;

      const doScroll = () => {
        if (hasScrolledRef.current === targetConversationId) return;
        virtuosoRef.current?.scrollToIndex({ index: timeline.length - 1, align: "end", behavior: "auto" });
        hasScrolledRef.current = targetConversationId;
      };

      const scroller = scrollerElRef.current;
      if (!scroller) {
        // Fallback: scroller not yet mounted, use fixed delay
        const timer = setTimeout(doScroll, MARKDOWN_RENDER_DELAY_MS);
        return () => clearTimeout(timer);
      }

      let debounceTimer: ReturnType<typeof setTimeout>;
      const observer = new ResizeObserver(() => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
          doScroll();
          observer.disconnect();
        }, 200);
      });

      observer.observe(scroller);

      // Safety timeout: 3s max — disconnect + force scroll if debounce never settles
      const safetyTimer = setTimeout(() => {
        observer.disconnect();
        doScroll();
      }, 3000);

      return () => {
        observer.disconnect();
        clearTimeout(debounceTimer);
        clearTimeout(safetyTimer);
      };
    }, [conversationId, timeline.length]);

    // Memoize Virtuoso components to prevent infinite re-render loop.
    // Inline object literals create new references every render, causing Virtuoso
    // to re-mount Header/Footer → layout change → atBottomStateChange → re-render → loop.
    const virtuosoComponents = useMemo(() => ({
      Header: () => (
        <div className="px-3 pt-3 w-full" style={contentContainerStyle}>
          {/* Show failed run banner if last run failed */}
          {failedRun?.errorMessage && onDismissFailedRun && (
            <FailedRunBanner
              errorMessage={failedRun.errorMessage}
              onDismiss={() => onDismissFailedRun(failedRun.id)}
            />
          )}
        </div>
      ),
      Footer: () => {
        return (
          <div ref={handleFooterRef} className="px-3 pb-3 w-full relative" style={contentContainerStyle}>
            {/* Render streaming content blocks in order — text, tool calls, and Task cards interleaved */}
            {streamingContentBlocks && streamingContentBlocks.map((block, idx) => {
              if (block.type === "text") {
                // Skip empty/whitespace-only text blocks (e.g. pre-stream flush artifacts)
                if (!block.text.trim()) return null;
                return (
                  <MessageItem
                    key={`streaming-text-${idx}`}
                    role="assistant"
                    content={block.text}
                    createdAt={new Date().toISOString()}
                    toolCalls={null}
                    contentBlocks={null}
                  />
                );
              }
              // task position marker — renders TaskSubagentCard at its chronological position.
              // Task metadata may not be available yet (agent:task_started fires after agent:tool_call),
              // so render nothing gracefully when the map entry is missing.
              if (block.type === "task") {
                const task = streamingTasks?.get(block.toolUseId);
                if (!task) return null;
                return <TaskSubagentCard key={`streaming-task-${block.toolUseId}`} task={task} />;
              }
              // tool_use block — diff calls render as DiffToolCallView, all others render as ToolCallIndicator
              if (isDiffToolCall(block.toolCall.name) && block.toolCall.arguments != null) {
                return (
                  <DiffToolCallView
                    key={`streaming-tool-${idx}`}
                    toolCall={block.toolCall}
                    isStreaming={block.toolCall.result == null && !block.toolCall.error}
                    className="mb-2"
                  />
                );
              }
              // Non-diff tool call — render inline to preserve visual ordering with text blocks
              return (
                <ToolCallIndicator
                  key={`streaming-tool-${idx}`}
                  toolCall={block.toolCall}
                  isStreaming={block.toolCall.result == null && !block.toolCall.error}
                  className="mb-2"
                />
              );
            })}

            {/* Fallback when agent is running but no content blocks yet:
                - Tool calls pending → show ToolCallIndicator for each (immediate visibility into what agent is doing)
                - No tool calls either → show TypingIndicator (agent thinking) */}
            {(isSending || isAgentRunning) && (!streamingContentBlocks || streamingContentBlocks.length === 0) && (
              streamingToolCalls.length > 0
                ? streamingToolCalls.map((tc, idx) => (
                    <ToolCallIndicator
                      key={`pending-tool-${idx}`}
                      toolCall={tc}
                      isStreaming={tc.result == null && !tc.error}
                      className="mb-2"
                    />
                  ))
                : <TypingIndicator />
            )}

          </div>
        );
      },
    }), [
      failedRun, onDismissFailedRun,
      streamingToolCalls, streamingTasks, streamingContentBlocks,
      isSending, isAgentRunning, handleFooterRef,
    ]);

    // Detect when a teammate tab filter produces zero timeline items but messages exist.
    const isFilteredTabEmpty = teamFilter && teamFilter !== "lead" && timeline.length === 0 && messages.length > 0;
    const emptyTabLabel = isFilteredTabEmpty
      ? (teamFilter === "lead" ? "Lead" : teamFilter)
      : null;

    // Helper to look up teammate info from team store
    const getTeammateInfo = useCallback((sender: string | null | undefined) => {
      if (!sender || !contextKey) {
        return { teammateName: null, teammateColor: null };
      }
      const selector = selectTeammateByName(contextKey, sender);
      const teammate = selector(useTeamStore.getState());
      return {
        teammateName: teammate?.name ?? null,
        teammateColor: teammate?.color ?? null,
      };
    }, [contextKey]);

    // Memoize itemContent — lookup teammate info for team mode messages
    const renderItem = useCallback((_: number, item: TimelineItem) => {
      if (item.kind === "hook") {
        return (
          <div className="px-3 w-full" style={contentContainerStyle}>
            <HookEventMessage event={item.data} />
          </div>
        );
      }
      if (item.kind === "team_event") {
        const teamMsg = item.data;
        return (
          <div className="px-3 w-full" style={contentContainerStyle}>
            <TeamMessageBubble
              from={teamMsg.from}
              to={teamMsg.to}
              content={teamMsg.content}
              timestamp={teamMsg.timestamp}
            />
          </div>
        );
      }
      const msg = item.data;

      // Render auto-verification messages as system cards, not user bubbles
      if (msg.metadata) {
        try {
          const meta = JSON.parse(msg.metadata) as Record<string, unknown>;
          if (meta[AUTO_VERIFICATION_KEY]) {
            return (
              <div className="px-3 w-full" style={contentContainerStyle}>
                <AutoVerificationCard content={msg.content} createdAt={msg.createdAt} />
              </div>
            );
          }
        } catch { /* not JSON, render normally */ }
      }

      // Look up teammate info if sender is present and message is from assistant
      const { teammateName, teammateColor } = msg.role === "assistant"
        ? getTeammateInfo(msg.sender)
        : { teammateName: null, teammateColor: null };

      return (
        <div className="px-3 w-full" style={contentContainerStyle}>
          <MessageItem
            role={msg.role}
            content={msg.content}
            createdAt={msg.createdAt}
            toolCalls={msg.toolCalls ?? null}
            contentBlocks={msg.contentBlocks ?? null}
            {...(msg.attachments && { attachments: msg.attachments })}
            teammateName={teammateName}
            teammateColor={teammateColor}
          />
        </div>
      );
    }, [getTeammateInfo]);

    if (isTestEnv) {
      return (
        <div className="flex-1 overflow-hidden relative" data-testid="integrated-chat-messages">
          {isFilteredTabEmpty && (
            <div className="flex-1 flex items-center justify-center h-full" data-testid="teammate-tab-empty">
              <span className="text-sm" style={{ color: "hsl(220 10% 40%)" }}>
                No messages from {emptyTabLabel} yet
              </span>
            </div>
          )}
          <div className="px-3 pt-3 w-full" style={contentContainerStyle}>
            {failedRun?.errorMessage && onDismissFailedRun && (
              <FailedRunBanner
                errorMessage={failedRun.errorMessage}
                onDismiss={() => onDismissFailedRun(failedRun.id)}
              />
            )}
          </div>

          {timeline.map((item, index) => {
            if (item.kind === "hook") {
              return (
                <div key={`${item.kind}-${item.sortTime}-${index}`} className="px-3 w-full" style={contentContainerStyle}>
                  <HookEventMessage event={item.data} />
                </div>
              );
            }
            if (item.kind === "team_event") {
              const teamMsg = item.data;
              return (
                <div key={`team-${teamMsg.id}`} className="px-3 w-full" style={contentContainerStyle}>
                  <TeamMessageBubble
                    from={teamMsg.from}
                    to={teamMsg.to}
                    content={teamMsg.content}
                    timestamp={teamMsg.timestamp}
                  />
                </div>
              );
            }
            const msg = item.data;

            // Render auto-verification messages as system cards, not user bubbles
            if (msg.metadata) {
              try {
                const meta = JSON.parse(msg.metadata) as Record<string, unknown>;
                if (meta[AUTO_VERIFICATION_KEY]) {
                  return (
                    <div key={`${item.kind}-${item.sortTime}-${index}`} className="px-3 w-full" style={contentContainerStyle}>
                      <AutoVerificationCard content={msg.content} createdAt={msg.createdAt} />
                    </div>
                  );
                }
              } catch { /* not JSON, render normally */ }
            }

            const { teammateName, teammateColor } = msg.role === "assistant"
              ? getTeammateInfo(msg.sender)
              : { teammateName: null, teammateColor: null };

            return (
              <div key={`${item.kind}-${item.sortTime}-${index}`} className="px-3 w-full" style={contentContainerStyle}>
                <MessageItem
                  role={msg.role}
                  content={msg.content}
                  createdAt={msg.createdAt}
                  toolCalls={msg.toolCalls ?? null}
                  contentBlocks={msg.contentBlocks ?? null}
                  {...(msg.attachments && { attachments: msg.attachments })}
                  teammateName={teammateName}
                  teammateColor={teammateColor}
                />
              </div>
            );
          })}

          <div className="px-3 pb-3 w-full" style={contentContainerStyle}>
            {/* Render streaming content blocks in order — text, tool calls, and Task cards interleaved */}
            {streamingContentBlocks && streamingContentBlocks.map((block, idx) => {
              if (block.type === "text") {
                // Skip empty/whitespace-only text blocks (e.g. pre-stream flush artifacts)
                if (!block.text.trim()) return null;
                return (
                  <MessageItem
                    key={`streaming-text-${idx}`}
                    role="assistant"
                    content={block.text}
                    createdAt={new Date().toISOString()}
                    toolCalls={null}
                    contentBlocks={null}
                  />
                );
              }
              // task position marker — renders TaskSubagentCard at its chronological position
              if (block.type === "task") {
                const task = streamingTasks?.get(block.toolUseId);
                if (!task) return null;
                return <TaskSubagentCard key={`streaming-task-${block.toolUseId}`} task={task} />;
              }
              // tool_use block — diff calls render as DiffToolCallView, all others render as ToolCallIndicator
              if (isDiffToolCall(block.toolCall.name) && block.toolCall.arguments != null) {
                return (
                  <DiffToolCallView
                    key={`streaming-tool-${idx}`}
                    toolCall={block.toolCall}
                    isStreaming={block.toolCall.result == null && !block.toolCall.error}
                    className="mb-2"
                  />
                );
              }
              // Non-diff tool call — render inline to preserve visual ordering with text blocks
              return (
                <ToolCallIndicator
                  key={`streaming-tool-${idx}`}
                  toolCall={block.toolCall}
                  isStreaming={block.toolCall.result == null && !block.toolCall.error}
                  className="mb-2"
                />
              );
            })}

            {/* Fallback when agent is running but no content blocks yet:
                - Tool calls pending → show ToolCallIndicator for each (immediate visibility into what agent is doing)
                - No tool calls either → show TypingIndicator (agent thinking) */}
            {(isSending || isAgentRunning) && (!streamingContentBlocks || streamingContentBlocks.length === 0) && (
              streamingToolCalls.length > 0
                ? streamingToolCalls.map((tc, idx) => (
                    <ToolCallIndicator
                      key={`pending-tool-${idx}`}
                      toolCall={tc}
                      isStreaming={tc.result == null && !tc.error}
                      className="mb-2"
                    />
                  ))
                : <TypingIndicator />
            )}
            <div ref={messagesEndRef} />
          </div>
          {/* Scroll-to-bottom button — same position as production branch */}
          {!isAtBottom && timeline.length > 5 && !scrollToTimestamp && (
            <div className="absolute bottom-4 left-0 right-0 flex justify-center z-10 pointer-events-none">
              <Button
                variant="outline"
                size="sm"
                onClick={scrollToBottom}
                className="bg-background/95 backdrop-blur shadow-md hover:bg-accent pointer-events-auto"
              >
                <ChevronDown className="h-4 w-4 mr-1" />
                Scroll to bottom
              </Button>
            </div>
          )}
        </div>
      );
    }

    return (
      <ToolCallStoreKeyContext.Provider value={contextKey ?? null}>
      <div className="flex-1 overflow-hidden relative" data-testid="integrated-chat-messages">
        {isFilteredTabEmpty && (
          <div className="absolute inset-0 flex items-center justify-center" data-testid="teammate-tab-empty">
            <span className="text-sm" style={{ color: "hsl(220 10% 40%)" }}>
              No messages from {emptyTabLabel} yet
            </span>
          </div>
        )}
        <Virtuoso
          // Key forces complete remount when conversation changes - prevents scroll animation conflicts
          key={conversationId ?? "empty"}
          ref={virtuosoRef}
          scrollerRef={handleScrollerRef}
          data={timeline}
          context={footerContentHash}
          // Start at the last item on mount
          initialTopMostItemIndex={timeline.length > 0 ? timeline.length - 1 : 0}
          followOutput={handleFollowOutput}
          atBottomStateChange={handleAtBottomStateChange}
          atBottomThreshold={AT_BOTTOM_THRESHOLD}
          alignToBottom
          className="h-full"
          components={virtuosoComponents}
          itemContent={renderItem}
        />
        {/* Scroll-to-bottom button — OUTSIDE Virtuoso to avoid Footer feedback loop.
            isAtBottom/scrollToBottom/timeline.length are NOT in virtuosoComponents deps. */}
        {!isAtBottom && timeline.length > 5 && !scrollToTimestamp && (
          <div className="absolute bottom-4 left-0 right-0 flex justify-center z-10 pointer-events-none">
            <Button
              variant="outline"
              size="sm"
              onClick={scrollToBottom}
              className="bg-background/95 backdrop-blur shadow-md hover:bg-accent pointer-events-auto"
            >
              <ChevronDown className="h-4 w-4 mr-1" />
              Scroll to bottom
            </Button>
          </div>
        )}
      </div>
      </ToolCallStoreKeyContext.Provider>
    );
  }
);
