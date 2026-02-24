/**
 * ChatMessageList - Virtualized message list for chat panels
 *
 * Wraps react-virtuoso with chat-specific rendering:
 * - Auto-scroll to bottom
 * - Failed run banner header
 * - Worker executing indicator
 * - Streaming tool calls / typing indicator footer
 */

import React, { forwardRef, useCallback, useEffect, useMemo, useRef, useImperativeHandle } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { MessageItem } from "./MessageItem";
import { HookEventMessage } from "./HookEventMessage";
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
import { useMessageAttachments } from "@/hooks/useMessageAttachments";
import { ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { MessageAttachment } from "./MessageAttachments";
import { useTeamStore, selectTeammateByName, selectTeamMessages, EMPTY_TEAM_MESSAGES } from "@/stores/teamStore";
import type { TeamMessage } from "@/stores/teamStore";
import { TeamMessageBubble } from "./TeamMessageBubble";

// ============================================================================
// Constants
// ============================================================================

/** Delay for markdown content to render and expand before scroll correction */
const MARKDOWN_RENDER_DELAY_MS = 300;

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
  teamFilter?: "all" | "lead" | string | undefined;
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
    // Internal ref for scroll operations
    const virtuosoRef = useRef<VirtuosoHandle>(null);
    const hasScrolledRef = useRef<string | null>(null);
    const isTestEnv = import.meta.env.VITEST;

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

    const footerContentHash = useMemo(() => ({
      toolCallCount: streamingToolCalls.length,
      childCallCount: totalChildCalls,
      taskCount: streamingTasks?.size ?? 0,
      contentBlockCount: streamingContentBlocks?.length ?? 0,
    }), [streamingToolCalls.length, totalChildCalls, streamingTasks?.size, streamingContentBlocks?.length]);

    // Streaming auto-scroll — followOutput only fires on totalCount changes,
    // NOT on Footer height growth. Call autoscrollToBottom() imperatively when
    // footer content changes to keep the view pinned during streaming.
    useEffect(() => {
      if (scrollToTimestamp) return; // Don't auto-scroll in history mode
      virtuosoRef.current?.autoscrollToBottom();
    }, [footerContentHash, scrollToTimestamp]);

    // Unified auto-scroll hook — Virtuoso followOutput handles new-message scroll,
    // while the useEffect above handles streaming footer growth.
    const {
      messagesEndRef,
      isAtBottom,
      scrollToBottom,
      handleAtBottomStateChange,
      handleFollowOutput,
    } = useChatAutoScroll({
      messageCount: messages.length,
      disabled: !!scrollToTimestamp, // Disable auto-scroll in history mode
      virtuosoRef, // Route scrollToBottom through Virtuoso scrollToIndex
      conversationId, // Reset isAtBottom when conversation changes
    });

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
            behavior: "smooth",
          });
        }, MARKDOWN_RENDER_DELAY_MS);
        return () => clearTimeout(timeoutId);
      }
      return undefined;
    }, [scrollToTimestamp, messages]);

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

    const timeline = useMemo((): TimelineItem[] => {
      const items: TimelineItem[] = [];

      // Exclude the last assistant message from DB when:
      // (a) active streaming/finalizing — it's being rendered in streamingContentBlocks
      // (b) agent is running + last assistant is empty — prevents empty "pill" before first chunk
      const filteredMessages = (shouldFilterLastAssistant || isAgentRunning)
        ? (() => {
            // Find the last assistant message index
            let lastAssistantIdx = -1;
            for (let i = messages.length - 1; i >= 0; i--) {
              if (messages[i]!.role === "assistant") {
                lastAssistantIdx = i;
                break;
              }
            }
            if (lastAssistantIdx >= 0) {
              const lastMsg = messages[lastAssistantIdx]!;
              // Always filter when streaming/finalizing; only filter empty msg when only running
              if (shouldFilterLastAssistant || !lastMsg.content.trim()) {
                return messages.filter((_, idx) => idx !== lastAssistantIdx);
              }
            }
            return messages;
          })()
        : messages;

      // Apply team filter if active
      const teamFilteredMessages = teamFilter && teamFilter !== "all"
        ? filteredMessages.filter((msg) => {
            // User messages always show in every tab (directed at lead, relevant context for all views)
            if (msg.role === "user") return true;
            if (teamFilter === "lead") {
              // Show lead messages only (sender === "lead")
              return msg.sender === "lead";
            }
            // Show messages from specific teammate only; null-sender system/assistant messages only in "all" tab
            return msg.sender === teamFilter;
          })
        : filteredMessages;

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
        const filteredTeamMsgs = teamFilter && teamFilter !== "all"
          ? teamMessages.filter((msg) => {
              if (teamFilter === "lead") {
                return msg.from === "lead" || msg.to === "lead" || msg.from === "system" || msg.from === "user";
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
    }, [messages, hookEvents, activeHooks, hasHookEvents, shouldFilterLastAssistant, isAgentRunning, streamingContentBlocks, streamingTasks, conversationId, attachmentsMap, teamFilter, teamMessages]);

    // Explicit initial scroll — fires when conversation changes to ensure
    // the last message is visible after layout settles.
    // Virtuoso's initialTopMostItemIndex races with layout calculation on mount,
    // so we use a delayed scrollToIndex to guarantee scroll position is correct.
    // Uses hasScrolledRef to scroll exactly once per conversation (when timeline
    // transitions from empty to populated after async data load).
    useEffect(() => {
      if (timeline.length === 0 || !conversationId || isTestEnv) return;
      if (hasScrolledRef.current === conversationId) return;

      const timeoutId = setTimeout(() => {
        hasScrolledRef.current = conversationId;
        virtuosoRef.current?.scrollToIndex({
          index: timeline.length - 1,
          align: "end",
          behavior: "auto", // 'auto' = instant jump (no animation)
        });
      }, MARKDOWN_RENDER_DELAY_MS);

      return () => clearTimeout(timeoutId);
    }, [conversationId, timeline.length, isTestEnv]);

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
          <div className="px-3 pb-3 w-full relative" style={contentContainerStyle}>
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

            {/* Typing indicator — shows when thinking but no content blocks or tool calls are active yet.
                StreamingToolIndicator is now rendered outside the scroll container by parent panels. */}
            {(isSending || isAgentRunning) && streamingToolCalls.length === 0 && (!streamingContentBlocks || streamingContentBlocks.length === 0) && (
              <TypingIndicator />
            )}

          </div>
        );
      },
    }), [
      failedRun, onDismissFailedRun,
      streamingToolCalls.length, streamingTasks, streamingContentBlocks,
      isSending, isAgentRunning,
    ]);

    // Detect when a teammate tab filter produces zero results but messages exist
    // Show an empty state placeholder instead of a blank view
    const isFilteredTabEmpty = teamFilter && teamFilter !== "all" && timeline.length === 0 && messages.length > 0;
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

            {/* Typing indicator — shows when thinking but no content blocks or tool calls are active yet.
                StreamingToolIndicator is now rendered outside the scroll container by parent panels. */}
            {(isSending || isAgentRunning) && streamingToolCalls.length === 0 && (!streamingContentBlocks || streamingContentBlocks.length === 0) && (
              <TypingIndicator />
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
          data={timeline}
          context={footerContentHash}
          // Start at the last item on mount
          initialTopMostItemIndex={timeline.length > 0 ? timeline.length - 1 : 0}
          followOutput={handleFollowOutput}
          atBottomStateChange={handleAtBottomStateChange}
          atBottomThreshold={150}
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
    );
  }
);
