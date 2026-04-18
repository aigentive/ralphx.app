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
import { VerificationResultCard } from "./VerificationResultCard";
import { AUTO_VERIFICATION_KEY, VERIFICATION_RESULT_KEY } from "@/types/ideation";
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
import { isProviderRole } from "@/lib/chat/provider-role";
import { normalizeStreamingVerificationContentBlocks } from "./verification-tool-calls";

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
  providerHarness?: string | null;
  providerSessionId?: string | null;
  upstreamProvider?: string | null;
  providerProfile?: string | null;
  logicalModel?: string | null;
  effectiveModelId?: string | null;
  logicalEffort?: string | null;
  effectiveEffort?: string | null;
  inputTokens?: number | null;
  outputTokens?: number | null;
  cacheCreationTokens?: number | null;
  cacheReadTokens?: number | null;
  estimatedUsd?: number | null;
}

/** Discriminated union for timeline items when hook events are interleaved */
type TimelineItem =
  | { kind: "message"; data: ChatMessageData; sortTime: number }
  | { kind: "hook"; data: HookEvent | HookStartedEvent; sortTime: number }
  | { kind: "team_event"; data: TeamMessage; sortTime: number };

function parseMessageMetadata(metadata: string | null | undefined): Record<string, unknown> | null {
  if (!metadata) return null;
  try {
    return JSON.parse(metadata) as Record<string, unknown>;
  } catch {
    return null;
  }
}

function renderSystemCard(
  metadata: Record<string, unknown> | null,
  content: string,
  createdAt: string,
) {
  if (!metadata) return null;

  if (metadata[AUTO_VERIFICATION_KEY]) {
    return <AutoVerificationCard content={content} createdAt={createdAt} />;
  }

  if (metadata[VERIFICATION_RESULT_KEY]) {
    const blockers = Array.isArray(metadata.top_blockers)
      ? metadata.top_blockers
          .filter((item): item is { severity?: unknown; description?: unknown } => (
            item != null && typeof item === "object"
          ))
          .map((item) => ({
            severity: typeof item.severity === "string" ? item.severity : "unknown",
            description: typeof item.description === "string" ? item.description : "",
          }))
          .filter((item) => item.description.length > 0)
      : [];

    return (
      <VerificationResultCard
        summary={typeof metadata.summary === "string" ? metadata.summary : content}
        convergenceReason={typeof metadata.convergence_reason === "string" ? metadata.convergence_reason : null}
        currentRound={typeof metadata.current_round === "number" ? metadata.current_round : null}
        maxRounds={typeof metadata.max_rounds === "number" ? metadata.max_rounds : null}
        recommendedNextAction={
          typeof metadata.recommended_next_action === "string"
            ? metadata.recommended_next_action
            : null
        }
        blockers={blockers}
        actionableForParent={metadata.actionable_for_parent === true}
      />
    );
  }

  return null;
}

interface ChatMessageListProps {
  messages: ChatMessageData[];
  /** Conversation ID - used as key to force remount on conversation switch */
  conversationId: string | null;
  /** Absolute index of the first loaded message in the full conversation timeline */
  firstItemIndex?: number;
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
  /** Provider metadata for the active conversation */
  providerHarness?: string | null | undefined;
  providerSessionId?: string | null | undefined;
  hasOlderMessages?: boolean;
  isFetchingOlderMessages?: boolean;
  onLoadOlderMessages?: (() => void | Promise<void>) | undefined;
}

// ============================================================================
// Component
// ============================================================================

export const ChatMessageList = forwardRef<VirtuosoHandle, ChatMessageListProps>(
  function ChatMessageList(
    {
      messages,
      conversationId,
      firstItemIndex = 0,
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
      providerHarness,
      providerSessionId,
      hasOlderMessages = false,
      isFetchingOlderMessages = false,
      onLoadOlderMessages,
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
    const normalizedStreamingContentBlocks = useMemo(
      () => normalizeStreamingVerificationContentBlocks(streamingContentBlocks),
      [streamingContentBlocks],
    );

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
      if (!normalizedStreamingContentBlocks.length) {
        setCumulativeTextLength(0);
        return;
      }
      const total = normalizedStreamingContentBlocks.reduce(
        (sum, block) => block.type === "text" ? sum + block.text.length : sum, 0
      );
      setCumulativeTextLength(prev => Math.max(prev, total));
    }, [normalizedStreamingContentBlocks]);

    const hasRenderableStreamingBlocks = useMemo(
      () =>
        normalizedStreamingContentBlocks.some((block) => {
          if (block.type === "text") {
            return block.text.trim().length > 0;
          }
          if (block.type === "task") {
            return Boolean(streamingTasks?.get(block.toolUseId));
          }
          return true;
        }),
      [normalizedStreamingContentBlocks, streamingTasks],
    );

    const shouldShowFooterFallback = (isSending || isAgentRunning) && !hasRenderableStreamingBlocks;
    const hasFooterStreamingContent = hasRenderableStreamingBlocks || shouldShowFooterFallback;

    useEffect(() => {
      hasFooterStreamingContentRef.current = hasFooterStreamingContent;
    }, [hasFooterStreamingContent]);

    const footerContentHash = useMemo(() => ({
      toolCallCount: streamingToolCalls.length,
      // G1 fix: results update existing blocks (count unchanged) — track result arrivals separately
      toolResultCount: streamingToolCalls.filter(tc => tc.result != null || tc.error != null).length,
      childCallCount: totalChildCalls,
      taskCount: streamingTasks?.size ?? 0,
      contentBlockCount: normalizedStreamingContentBlocks.length,
      textLengthBucket: Math.floor(cumulativeTextLength / TEXT_LENGTH_BUCKET_SIZE),
    }), [streamingToolCalls, totalChildCalls, streamingTasks?.size, normalizedStreamingContentBlocks.length, cumulativeTextLength]);

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
      indexOffset: firstItemIndex,
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
            index: firstItemIndex + targetIndex,
            align: "start",
            behavior: preferredScrollBehavior,
          });
        }, MARKDOWN_RENDER_DELAY_MS);
        return () => clearTimeout(timeoutId);
      }
      return undefined;
    }, [scrollToTimestamp, messages, firstItemIndex, preferredScrollBehavior]);

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
    const hasActiveStreaming = normalizedStreamingContentBlocks.length > 0 ||
                              (streamingTasks && streamingTasks.size > 0);
    const shouldFilterLastProviderMessage = hasActiveStreaming || isFinalizing;

    // When filter clears (streaming/finalizing ends), scroll to bottom so the newly
    // revealed finalized assistant message is visible.
    useEffect(() => {
      if (scrollToTimestamp) return; // Don't auto-scroll in history mode
      if (prevShouldFilterRef.current && !shouldFilterLastProviderMessage) {
        scrollToBottom();
      }
      prevShouldFilterRef.current = shouldFilterLastProviderMessage;
    }, [shouldFilterLastProviderMessage, scrollToBottom, scrollToTimestamp]);

    const timeline = useMemo((): TimelineItem[] => {
      const items: TimelineItem[] = [];

      // Exclude the streaming assistant message from DB when active streaming/finalizing —
      // it's being rendered live in streamingContentBlocks. Do NOT filter based solely on
      // isAgentRunning: during team sessions the lead runs for extended periods, and filtering
      // without active streaming blocks hides historical assistant messages between turns.
      //
      // Use ID-based filtering: find the assistant message with the most recent createdAt
      // (with id as tiebreaker) so filtering is stable regardless of array order.
      const filteredMessages = shouldFilterLastProviderMessage
        ? (() => {
            // Find the most recently created provider message by timestamp (stable, not index)
            let latestProviderMessageId: string | null = null;
            let latestProviderMessageTime = -Infinity;
            for (const msg of messages) {
              if (isProviderRole(msg.role)) {
                const t = new Date(msg.createdAt).getTime();
                if (
                  t > latestProviderMessageTime ||
                  (t === latestProviderMessageTime && msg.id > (latestProviderMessageId ?? ""))
                ) {
                  latestProviderMessageTime = t;
                  latestProviderMessageId = msg.id;
                }
              }
            }
            if (latestProviderMessageId !== null) {
              return messages.filter((msg) => msg.id !== latestProviderMessageId);
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
    }, [messages, hookEvents, activeHooks, hasHookEvents, shouldFilterLastProviderMessage, attachmentsMap, teamFilter, teamMessages]);

    const lastItemIndex = firstItemIndex + timeline.length - 1;
    const startReachedHandler =
      hasOlderMessages && onLoadOlderMessages
        ? (_index: number) => {
            void onLoadOlderMessages();
          }
        : null;

    // Initial load scroll — fires when conversation changes and timeline populates.
    // Uses one-shot ResizeObserver on the scroller element to detect when virtual
    // content has actually rendered, rather than a fixed-duration setTimeout guess.
    // Falls back to MARKDOWN_RENDER_DELAY_MS if scrollerElRef not yet available.
    useEffect(() => {
      const targetScrollKey =
        conversationId != null && lastItemIndex >= 0
          ? `${conversationId}:${lastItemIndex}`
          : null;

      if (!conversationId || timeline.length === 0 || hasScrolledRef.current === targetScrollKey) {
        return;
      }

      const doScroll = () => {
        if (hasScrolledRef.current === targetScrollKey) return;
        virtuosoRef.current?.scrollToIndex({
          index: lastItemIndex,
          align: "end",
          behavior: "auto",
        });
        hasScrolledRef.current = targetScrollKey;
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
    }, [conversationId, lastItemIndex, timeline.length]);

    const footerContent = useMemo(() => {
      if (!hasFooterStreamingContent) {
        return null;
      }

      return (
        <>
          {normalizedStreamingContentBlocks.map((block, idx) => {
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
                  providerHarness={providerHarness}
                  providerSessionId={providerSessionId}
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
          {shouldShowFooterFallback && (
            <>
              {streamingToolCalls.length > 0 && streamingToolCalls.map((tc, idx) => (
                <ToolCallIndicator
                  key={`pending-tool-${idx}`}
                  toolCall={tc}
                  isStreaming={tc.result == null && !tc.error}
                  className="mb-2"
                />
              ))}
              <TypingIndicator />
            </>
          )}
        </>
      );
    }, [
      hasFooterStreamingContent,
      normalizedStreamingContentBlocks,
      providerHarness,
      providerSessionId,
      shouldShowFooterFallback,
      streamingTasks,
      streamingToolCalls,
    ]);

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
        if (!footerContent) {
          return null;
        }
        return (
          <div ref={handleFooterRef} className="px-3 pb-3 w-full relative" style={contentContainerStyle}>
            {footerContent}
          </div>
        );
      },
    }), [
      failedRun, onDismissFailedRun,
      footerContent, handleFooterRef,
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
    const renderItem = useCallback((index: number, item: TimelineItem) => {
      const isLastTimelineItem = index === timeline.length - 1;
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

      const systemCard = renderSystemCard(
        parseMessageMetadata(msg.metadata),
        msg.content,
        msg.createdAt,
      );
      if (systemCard) {
        return (
          <div className="px-3 w-full" style={contentContainerStyle}>
            {systemCard}
          </div>
        );
      }

      // Look up teammate info if sender is present and message is from assistant
      const { teammateName, teammateColor } = isProviderRole(msg.role)
        ? getTeammateInfo(msg.sender)
        : { teammateName: null, teammateColor: null };

      return (
        <div className="px-3 w-full" style={contentContainerStyle}>
          <MessageItem
            role={msg.role}
            content={msg.content}
            createdAt={msg.createdAt}
            isLastInList={isLastTimelineItem}
            toolCalls={msg.toolCalls ?? null}
            contentBlocks={msg.contentBlocks ?? null}
            {...(msg.attachments && { attachments: msg.attachments })}
            teammateName={teammateName}
            teammateColor={teammateColor}
            providerHarness={msg.providerHarness ?? providerHarness}
            providerSessionId={msg.providerSessionId ?? providerSessionId}
            upstreamProvider={msg.upstreamProvider}
            providerProfile={msg.providerProfile}
            logicalModel={msg.logicalModel}
            effectiveModelId={msg.effectiveModelId}
            logicalEffort={msg.logicalEffort}
            effectiveEffort={msg.effectiveEffort}
            inputTokens={msg.inputTokens}
            outputTokens={msg.outputTokens}
            cacheCreationTokens={msg.cacheCreationTokens}
            cacheReadTokens={msg.cacheReadTokens}
            estimatedUsd={msg.estimatedUsd}
          />
        </div>
      );
    }, [getTeammateInfo, providerHarness, providerSessionId, timeline.length]);

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

            const systemCard = renderSystemCard(
              parseMessageMetadata(msg.metadata),
              msg.content,
              msg.createdAt,
            );
            if (systemCard) {
              return (
                <div key={`${item.kind}-${item.sortTime}-${index}`} className="px-3 w-full" style={contentContainerStyle}>
                  {systemCard}
                </div>
              );
            }

            const { teammateName, teammateColor } = isProviderRole(msg.role)
              ? getTeammateInfo(msg.sender)
              : { teammateName: null, teammateColor: null };

            return (
              <div key={`${item.kind}-${item.sortTime}-${index}`} className="px-3 w-full" style={contentContainerStyle}>
                <MessageItem
                  role={msg.role}
                  content={msg.content}
                  createdAt={msg.createdAt}
                  isLastInList={index === timeline.length - 1}
                  toolCalls={msg.toolCalls ?? null}
                  contentBlocks={msg.contentBlocks ?? null}
                  {...(msg.attachments && { attachments: msg.attachments })}
                  teammateName={teammateName}
                  teammateColor={teammateColor}
                  providerHarness={msg.providerHarness ?? providerHarness}
                  providerSessionId={msg.providerSessionId ?? providerSessionId}
                  upstreamProvider={msg.upstreamProvider}
                  providerProfile={msg.providerProfile}
                  logicalModel={msg.logicalModel}
                  effectiveModelId={msg.effectiveModelId}
                  logicalEffort={msg.logicalEffort}
                  effectiveEffort={msg.effectiveEffort}
                  inputTokens={msg.inputTokens}
                  outputTokens={msg.outputTokens}
                  cacheCreationTokens={msg.cacheCreationTokens}
                  cacheReadTokens={msg.cacheReadTokens}
                  estimatedUsd={msg.estimatedUsd}
                />
              </div>
            );
          })}

          {footerContent && (
            <div className="px-3 pb-3 w-full" style={contentContainerStyle}>
              {footerContent}
              <div ref={messagesEndRef} />
            </div>
          )}
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
          firstItemIndex={firstItemIndex}
          context={footerContentHash}
          // Start at the last item on mount
          initialTopMostItemIndex={timeline.length > 0 ? lastItemIndex : 0}
          followOutput={handleFollowOutput}
          atBottomStateChange={handleAtBottomStateChange}
          atBottomThreshold={AT_BOTTOM_THRESHOLD}
          {...(startReachedHandler
            ? { startReached: startReachedHandler }
            : {})}
          alignToBottom
          className="h-full"
          components={virtuosoComponents}
          itemContent={renderItem}
        />
        {isFetchingOlderMessages && (
          <div className="absolute top-2 left-0 right-0 flex justify-center pointer-events-none">
            <span
              className="rounded-full px-3 py-1 text-[11px]"
              style={{
                backgroundColor: "hsla(220 15% 12% / 0.94)",
                border: "1px solid hsla(220 20% 100% / 0.06)",
                color: "hsl(220 10% 72%)",
              }}
            >
              Loading earlier messages...
            </span>
          </div>
        )}
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
