/**
 * ChatMessageList - Virtualized message list for chat panels
 *
 * Wraps react-virtuoso with chat-specific rendering:
 * - Auto-scroll to bottom
 * - Failed run banner header
 * - Worker executing indicator
 * - Streaming tool calls / typing indicator footer
 */

import React, { forwardRef, useCallback, useEffect, useMemo, useRef, useImperativeHandle, useState } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { MessageItem } from "./MessageItem";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { HookEventMessage } from "./HookEventMessage";
import {
  TypingIndicator,
  FailedRunBanner,
} from "./IntegratedChatPanel.components";
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
import { useTeamStore, selectTeammateByName } from "@/stores/teamStore";

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
  | { kind: "hook"; data: HookEvent | HookStartedEvent; sortTime: number };

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
  /** Ref to track conversation that's finalizing (between message_created and query refetch) */
  finalizingConversationRef?: React.MutableRefObject<string | null>;
  /** Team filter for message filtering (team mode) */
  teamFilter?: "all" | "lead" | string;
  /** Context key for team store lookup (team mode) */
  contextKey?: string;
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
      finalizingConversationRef,
      teamFilter,
      contextKey,
    },
    ref
  ) {
    // Internal ref for scroll operations
    const virtuosoRef = useRef<VirtuosoHandle>(null);
    const isTestEnv = import.meta.env.VITEST;

    // Forward the ref to parent
    useImperativeHandle(ref, () => virtuosoRef.current!, []);

    // Track finalizing state to avoid accessing ref during render
    const [isFinalizing, setIsFinalizing] = useState(false);
    useEffect(() => {
      setIsFinalizing(finalizingConversationRef?.current === conversationId);
    }, [finalizingConversationRef, conversationId]);

    // Fetch attachments for all messages
    const { data: attachmentsMap } = useMessageAttachments(messages, conversationId);

    // Footer content hash — makes Virtuoso aware of footer height changes
    // without manual scrollTo calls. Virtuoso re-evaluates followOutput when
    // context changes, triggering a smooth scroll if user is at bottom.
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

    // Unified auto-scroll hook — Virtuoso followOutput is the single scroll path.
    // No isStreaming/streamingHash needed: Virtuoso re-evaluates followOutput
    // when its context prop changes (footerContentHash), handling all streaming
    // scroll updates through one mechanism.
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
    // The finalizingConversationRef persists through the timing window where streaming state
    // is cleared but the query refetch hasn't completed yet. This prevents duplicates during
    // that critical window.
    const hasActiveStreaming = (streamingContentBlocks && streamingContentBlocks.length > 0) ||
                              (streamingTasks && streamingTasks.size > 0);
    const shouldFilterLastAssistant = hasActiveStreaming || isFinalizing;

    const timeline = useMemo((): TimelineItem[] => {
      const items: TimelineItem[] = [];

      // If we have active streaming OR conversation is finalizing, exclude the last assistant message
      // from DB (it's being rendered in streamingContentBlocks, or about to appear from DB)
      const filteredMessages = shouldFilterLastAssistant
        ? (() => {
            // Find the last assistant message index
            let lastAssistantIdx = -1;
            for (let i = messages.length - 1; i >= 0; i--) {
              if (messages[i]!.role === "assistant") {
                lastAssistantIdx = i;
                break;
              }
            }
            // If found, exclude it
            if (lastAssistantIdx >= 0) {
              return messages.filter((_, idx) => idx !== lastAssistantIdx);
            }
            return messages;
          })()
        : messages;

      // Apply team filter if active
      const teamFilteredMessages = teamFilter && teamFilter !== "all"
        ? filteredMessages.filter((msg) => {
            if (teamFilter === "lead") {
              // Show lead messages (no sender or sender === "lead")
              return !msg.sender || msg.sender === "lead";
            }
            // Show messages from specific teammate
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
        items.sort((a, b) => a.sortTime - b.sortTime);
      }

      return items;
    }, [messages, hookEvents, activeHooks, hasHookEvents, shouldFilterLastAssistant, streamingContentBlocks, streamingTasks, conversationId, attachmentsMap, teamFilter]);

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
        // Filter out Task tool calls — they're already represented by TaskSubagentCard
        // Also filter out diff tool calls (Edit/Write) — they're rendered in streamingContentBlocks
        const otherToolCalls = streamingToolCalls.filter(
          (tc) => tc.name.toLowerCase() !== "task" &&
                  (!isDiffToolCall(tc.name) || tc.arguments == null)
        );

        return (
          <div className="px-3 pb-3 w-full relative" style={contentContainerStyle}>
            {/* Render streaming content blocks in order — text and tool calls interleaved */}
            {streamingContentBlocks && streamingContentBlocks.map((block, idx) => {
              if (block.type === "text") {
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
              // tool_use block — render as diff view if it's Edit/Write, otherwise skip (handled by StreamingToolIndicator)
              if (isDiffToolCall(block.toolCall.name) && block.toolCall.arguments != null) {
                return (
                  <DiffToolCallView
                    key={`streaming-tool-${idx}`}
                    toolCall={block.toolCall}
                    isStreaming
                    className="mb-2"
                  />
                );
              }
              return null;
            })}

            {/* Task subagent cards — above everything else */}
            {streamingTasks && streamingTasks.size > 0 &&
              Array.from(streamingTasks.values()).map((task: StreamingTask) => (
                <TaskSubagentCard key={task.toolUseId} task={task} />
              ))
            }

            {/* Aggregated indicator for remaining tools, or typing indicator */}
            {(isSending || isAgentRunning) && (
              otherToolCalls.length > 0 ? (
                <StreamingToolIndicator toolCalls={otherToolCalls} isActive={true} />
              ) : (!streamingContentBlocks || streamingContentBlocks.length === 0) ? (
                <TypingIndicator />
              ) : null
            )}

          </div>
        );
      },
    }), [
      failedRun, onDismissFailedRun,
      streamingToolCalls, streamingTasks, streamingContentBlocks,
      isSending, isAgentRunning,
    ]);

    // Helper to look up teammate info from team store
    const getTeammateInfo = useCallback((sender: string | null | undefined) => {
      if (!sender || !contextKey) {
        return { teammateName: null, teammateColor: null };
      }
      const selector = selectTeammateByName(contextKey, sender);
      const teammate = useTeamStore.getState()(selector);
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
            {/* Render streaming content blocks in order — text and tool calls interleaved */}
            {streamingContentBlocks && streamingContentBlocks.map((block, idx) => {
              if (block.type === "text") {
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
              // tool_use block — render as diff view if it's Edit/Write
              if (isDiffToolCall(block.toolCall.name) && block.toolCall.arguments != null) {
                return (
                  <DiffToolCallView
                    key={`streaming-tool-${idx}`}
                    toolCall={block.toolCall}
                    isStreaming
                    className="mb-2"
                  />
                );
              }
              return null;
            })}

            {streamingTasks && streamingTasks.size > 0 &&
              Array.from(streamingTasks.values()).map((task: StreamingTask) => (
                <TaskSubagentCard key={task.toolUseId} task={task} />
              ))
            }

            {(isSending || isAgentRunning) && (
              streamingToolCalls.length > 0 ? (
                <StreamingToolIndicator toolCalls={streamingToolCalls} isActive={true} />
              ) : (!streamingContentBlocks || streamingContentBlocks.length === 0) ? (
                <TypingIndicator />
              ) : null
            )}
            <div ref={messagesEndRef} />
          </div>
          {/* Scroll-to-bottom button — same position as production branch */}
          {!isAtBottom && messages.length > 5 && !scrollToTimestamp && (
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
            isAtBottom/scrollToBottom/messages.length are NOT in virtuosoComponents deps. */}
        {!isAtBottom && messages.length > 5 && !scrollToTimestamp && (
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
