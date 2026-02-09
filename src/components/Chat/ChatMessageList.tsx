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
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import {
  TypingIndicator,
  FailedRunBanner,
} from "./IntegratedChatPanel.components";
import type { ToolCall } from "./ToolCallIndicator";
import type { StreamingTask } from "@/types/streaming-task";
import type { ContentBlockItem } from "./MessageItem";
import { isDiffToolCall } from "./DiffToolCallView.utils";
import { DiffToolCallView } from "./DiffToolCallView";
import { TaskSubagentCard } from "./TaskSubagentCard";

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
}

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
  /** Streaming assistant text from agent:chunk events */
  streamingText?: string;
  /** Ref to scroll to */
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  /** Optional timestamp to scroll to (for history mode) - scrolls to first message at or after this time */
  scrollToTimestamp?: string | null;
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
      streamingText,
      messagesEndRef,
      scrollToTimestamp,
    },
    ref
  ) {
    // Internal ref for scroll operations
    const virtuosoRef = useRef<VirtuosoHandle>(null);

    // Forward the ref to parent
    useImperativeHandle(ref, () => virtuosoRef.current!, []);

    // Track whether user is at the bottom — drives followOutput behavior
    const isAtBottomRef = useRef(true);
    const handleAtBottomStateChange = useCallback((atBottom: boolean) => {
      isAtBottomRef.current = atBottom;
    }, []);

    // followOutput callback: only auto-scroll when user is already at bottom
    const handleFollowOutput = useCallback((isAtBottom: boolean) => {
      if (isAtBottom) return "smooth" as const;
      return false as const;
    }, []);

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
      hasText: !!streamingText,
    }), [streamingToolCalls.length, totalChildCalls, streamingTasks?.size, streamingText]);

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

    return (
      <div className="flex-1 overflow-hidden" data-testid="integrated-chat-messages">
        <Virtuoso
          // Key forces complete remount when conversation changes - prevents scroll animation conflicts
          key={conversationId ?? "empty"}
          ref={virtuosoRef}
          data={messages}
          context={footerContentHash}
          // Start at the last message on mount
          initialTopMostItemIndex={messages.length > 0 ? messages.length - 1 : 0}
          followOutput={handleFollowOutput}
          atBottomStateChange={handleAtBottomStateChange}
          atBottomThreshold={150}
          alignToBottom
          className="h-full"
          components={{
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
              const topLevelToolCalls = streamingToolCalls.filter(
                (tc) => tc.name.toLowerCase() !== "task"
              );

              // Split Edit/Write tool calls (with arguments) for individual diff rendering
              const diffToolCalls = topLevelToolCalls.filter(
                (tc) => isDiffToolCall(tc.name) && tc.arguments != null
              );
              const otherToolCalls = topLevelToolCalls.filter(
                (tc) => !isDiffToolCall(tc.name) || tc.arguments == null
              );

              return (
                <div className="px-3 pb-3 w-full" style={contentContainerStyle}>
                  {/* Show streaming assistant text from agent:chunk events */}
                  {streamingText && (
                    <MessageItem
                      key="streaming-assistant"
                      role="assistant"
                      content={streamingText}
                      createdAt={new Date().toISOString()}
                      toolCalls={null}
                      contentBlocks={null}
                    />
                  )}

                  {/* Task subagent cards — above everything else */}
                  {streamingTasks && streamingTasks.size > 0 &&
                    Array.from(streamingTasks.values()).map((task: StreamingTask) => (
                      <TaskSubagentCard key={task.toolUseId} task={task} />
                    ))
                  }

                  {/* Diff views for Edit/Write — shown as individual cards */}
                  {diffToolCalls.map((tc) => (
                    <DiffToolCallView key={tc.id} toolCall={tc} isStreaming className="mb-2" />
                  ))}

                  {/* Aggregated indicator for remaining tools, or typing indicator */}
                  {(isSending || isAgentRunning) && (
                    otherToolCalls.length > 0 ? (
                      <StreamingToolIndicator toolCalls={otherToolCalls} isActive={true} />
                    ) : !streamingText && diffToolCalls.length === 0 ? (
                      <TypingIndicator />
                    ) : null
                  )}
                  <div ref={messagesEndRef} />
                </div>
              );
            },
          }}
          itemContent={(_, msg) => (
            <div className="px-3 w-full" style={contentContainerStyle}>
              <MessageItem
                key={msg.id}
                role={msg.role}
                content={msg.content}
                createdAt={msg.createdAt}
                toolCalls={msg.toolCalls ?? null}
                contentBlocks={msg.contentBlocks ?? null}
              />
            </div>
          )}
        />
      </div>
    );
  }
);
