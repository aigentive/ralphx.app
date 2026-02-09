/**
 * ChatMessageList - Virtualized message list for chat panels
 *
 * Wraps react-virtuoso with chat-specific rendering:
 * - Auto-scroll to bottom
 * - Failed run banner header
 * - Worker executing indicator
 * - Streaming tool calls / typing indicator footer
 */

import React, { forwardRef, useEffect, useRef, useImperativeHandle } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { MessageItem } from "./MessageItem";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { AskUserQuestionCard } from "./AskUserQuestionCard";
import {
  TypingIndicator,
  FailedRunBanner,
} from "./IntegratedChatPanel.components";
import type { ToolCall } from "./ToolCallIndicator";
import type { StreamingTask } from "@/types/streaming-task";
import type { ContentBlockItem } from "./MessageItem";
import type { AskUserQuestionPayload, AskUserQuestionResponse } from "@/types/ask-user-question";
import { isDiffToolCall } from "./DiffToolCallView.utils";
import { DiffToolCallView } from "./DiffToolCallView";

// ============================================================================
// Constants
// ============================================================================

/** Delay for markdown content to render and expand before scroll correction */
const MARKDOWN_RENDER_DELAY_MS = 300;

/** Delay for footer to render new tool calls before scrolling */
const FOOTER_RENDER_DELAY_MS = 50;

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
  /** Active question from agent (ask-user-question) */
  activeQuestion?: AskUserQuestionPayload | null | undefined;
  /** Callback when user submits an answer */
  onSubmitAnswer?: ((response: AskUserQuestionResponse) => void) | undefined;
  /** Whether answer submission is in progress */
  isSubmittingAnswer?: boolean | undefined;
  /** Summary of the previously answered question */
  answeredQuestion?: string | undefined;
  /** Called when user dismisses the active question */
  onDismissQuestion?: (() => void) | undefined;
  /** Called when user dismisses the answered summary */
  onDismissAnswered?: (() => void) | undefined;
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
      streamingText,
      messagesEndRef,
      scrollToTimestamp,
      activeQuestion,
      onSubmitAnswer,
      isSubmittingAnswer = false,
      answeredQuestion,
      onDismissQuestion,
      onDismissAnswered,
    },
    ref
  ) {
    // Internal ref for scroll operations
    const virtuosoRef = useRef<VirtuosoHandle>(null);

    // Forward the ref to parent
    useImperativeHandle(ref, () => virtuosoRef.current!, []);

    /** Scroll to absolute bottom of the list */
    const scrollToBottom = () => {
      virtuosoRef.current?.scrollTo({ top: Number.MAX_SAFE_INTEGER });
    };

    // Delayed scroll correction to handle markdown rendering height changes
    // Markdown content can expand after initial render, throwing off scroll position
    useEffect(() => {
      if (!conversationId || messages.length === 0) return;

      const timeoutId = setTimeout(scrollToBottom, MARKDOWN_RENDER_DELAY_MS);
      return () => clearTimeout(timeoutId);
    }, [conversationId, messages.length]);

    // Scroll to bottom when streaming tool calls change (footer height changes)
    // Virtuoso's followOutput only tracks data changes, not footer expansion
    useEffect(() => {
      if (streamingToolCalls.length === 0) return;

      const timeoutId = setTimeout(scrollToBottom, FOOTER_RENDER_DELAY_MS);
      return () => clearTimeout(timeoutId);
    }, [streamingToolCalls.length]);

    // Scroll to bottom when streaming text changes (footer content expanding)
    useEffect(() => {
      if (!streamingText) return;

      const timeoutId = setTimeout(scrollToBottom, FOOTER_RENDER_DELAY_MS);
      return () => clearTimeout(timeoutId);
    }, [streamingText]);

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
          // Start at the last message on mount
          initialTopMostItemIndex={messages.length > 0 ? messages.length - 1 : 0}
          followOutput="smooth"
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
              // Split Edit/Write tool calls (with arguments) for individual diff rendering
              const diffToolCalls = streamingToolCalls.filter(
                (tc) => isDiffToolCall(tc.name) && tc.arguments != null
              );
              const otherToolCalls = streamingToolCalls.filter(
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
                  {/* Show inline question card when agent asks a question or answered summary */}
                  {(activeQuestion || answeredQuestion) && onSubmitAnswer && (
                    <AskUserQuestionCard
                      question={activeQuestion ?? { requestId: "", taskId: "", header: "", question: "", options: [], multiSelect: false }}
                      onSubmit={onSubmitAnswer}
                      isSubmitting={isSubmittingAnswer}
                      answeredWith={answeredQuestion}
                      onDismiss={onDismissQuestion}
                      onDismissAnswered={onDismissAnswered}
                    />
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
