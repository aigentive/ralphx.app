/**
 * ChatMessageList - Virtualized message list for chat panels
 *
 * Wraps react-virtuoso with chat-specific rendering:
 * - Auto-scroll to bottom
 * - Failed run banner header
 * - Worker executing indicator
 * - Streaming tool calls / typing indicator footer
 */

import React, { forwardRef } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { MessageItem } from "./MessageItem";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import {
  TypingIndicator,
  WorkerExecutingIndicator,
  FailedRunBanner,
} from "./IntegratedChatPanel.components";
import type { ToolCall } from "./ToolCallIndicator";
import type { ContentBlockItem } from "./MessageItem";

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
  /** Show worker executing indicator in header */
  isExecutionMode: boolean;
  /** Show failed run banner */
  failedRun?: { id: string; errorMessage: string } | null;
  /** Callback when failed run banner is dismissed */
  onDismissFailedRun?: (runId: string) => void;
  /** Is agent currently sending/responding */
  isSending: boolean;
  isAgentRunning: boolean;
  /** Streaming tool calls to display */
  streamingToolCalls: ToolCall[];
  /** Ref to scroll to */
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
}

// ============================================================================
// Component
// ============================================================================

export const ChatMessageList = forwardRef<VirtuosoHandle, ChatMessageListProps>(
  function ChatMessageList(
    {
      messages,
      isExecutionMode,
      failedRun,
      onDismissFailedRun,
      isSending,
      isAgentRunning,
      streamingToolCalls,
      messagesEndRef,
    },
    ref
  ) {
    return (
      <div className="flex-1 overflow-hidden" data-testid="integrated-chat-messages">
        <Virtuoso
          ref={ref}
          data={messages}
          followOutput="smooth"
          alignToBottom
          className="h-full"
          components={{
            Header: () => (
              <div
                className="px-3 pt-3 w-full"
                style={{ maxWidth: "100%", overflowWrap: "break-word", wordBreak: "break-word" }}
              >
                {/* Show failed run banner if last run failed */}
                {failedRun?.errorMessage && onDismissFailedRun && (
                  <FailedRunBanner
                    errorMessage={failedRun.errorMessage}
                    onDismiss={() => onDismissFailedRun(failedRun.id)}
                  />
                )}

                {/* Show worker executing indicator when in execution mode */}
                {isExecutionMode && <WorkerExecutingIndicator />}
              </div>
            ),
            Footer: () => (
              <div
                className="px-3 pb-3 w-full"
                style={{ maxWidth: "100%", overflowWrap: "break-word", wordBreak: "break-word" }}
              >
                {/* Show streaming tool calls or typing indicator while agent is working */}
                {(isSending || isAgentRunning) && (
                  streamingToolCalls.length > 0 ? (
                    <StreamingToolIndicator toolCalls={streamingToolCalls} isActive={true} />
                  ) : (
                    <TypingIndicator />
                  )
                )}
                <div ref={messagesEndRef} />
              </div>
            ),
          }}
          itemContent={(_, msg) => (
            <div
              className="px-3 w-full"
              style={{ maxWidth: "100%", overflowWrap: "break-word", wordBreak: "break-word" }}
            >
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
