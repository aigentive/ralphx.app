/**
 * ChatMessages - Message rendering and display logic
 *
 * Renders messages and hook events interleaved chronologically.
 * Hook events appear as thin annotations between message bubbles.
 */

import { useMemo, useRef, type RefObject } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { MessageItem, type ContentBlockItem } from "./MessageItem";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { HookEventMessage } from "./HookEventMessage";
import { type ToolCall } from "./ToolCallIndicator";
import type { HookEvent, HookStartedEvent } from "@/types/hook-event";
import { Bot, MessageSquare, Loader2, Activity, X, ArrowDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useChatAutoScroll } from "@/hooks/useChatAutoScroll";

interface Message {
  id: string;
  role: string;
  content: string;
  createdAt: string;
  /** Pre-parsed tool calls array (parsed at API layer) */
  toolCalls: ToolCall[] | null | undefined;
  /** Pre-parsed content blocks array (parsed at API layer) */
  contentBlocks: ContentBlockItem[] | null | undefined;
}

/** Discriminated union for timeline items */
type TimelineItem =
  | { kind: "message"; data: Message; sortTime: number }
  | { kind: "hook"; data: HookEvent | HookStartedEvent; sortTime: number };

// ============================================================================
// Sub-components
// ============================================================================

function TypingIndicator() {
  return (
    <div
      data-testid="chat-typing-indicator"
      className="flex items-start gap-2 mb-2"
    >
      <Bot className="w-3.5 h-3.5 mt-2 shrink-0 text-white/40" />
      <div
        className="px-3 py-2 rounded-[10px_10px_10px_4px]"
        style={{
          background: "linear-gradient(180deg, rgba(28,28,28,0.95) 0%, rgba(22,22,22,0.98) 100%)",
          border: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        <div className="flex items-center gap-1">
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-white/30" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-white/30" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-white/30" />
        </div>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div
      data-testid="chat-panel-empty"
      className="flex flex-col items-center justify-center h-full p-6 text-center"
    >
      <div
        className="w-12 h-12 rounded-xl flex items-center justify-center mb-3"
        style={{
          background: "linear-gradient(135deg, rgba(255,107,53,0.1) 0%, rgba(255,107,53,0.05) 100%)",
          border: "1px solid rgba(255,107,53,0.15)",
        }}
      >
        <MessageSquare className="w-5 h-5 text-[#ff6b35]" />
      </div>
      <p className="text-[13px] font-medium text-white/80">
        Start a conversation
      </p>
      <p className="text-xs mt-1 text-white/40">
        Ask questions or get help with your tasks
      </p>
    </div>
  );
}

function LoadingState() {
  return (
    <div
      data-testid="chat-panel-loading"
      className="flex items-center justify-center p-6"
    >
      <Loader2 className="w-5 h-5 animate-spin text-[#ff6b35]" />
    </div>
  );
}

interface FailedRunBannerProps {
  errorMessage: string;
  onDismiss: (() => void) | undefined;
}

function FailedRunBanner({ errorMessage, onDismiss }: FailedRunBannerProps) {
  return (
    <div
      data-testid="failed-run-banner"
      className="flex items-start gap-2 px-3 py-2 mb-2 rounded-lg"
      style={{
        background: "linear-gradient(135deg, rgba(239,68,68,0.12) 0%, rgba(239,68,68,0.05) 100%)",
        border: "1px solid rgba(239,68,68,0.25)",
      }}
    >
      <Activity className="w-3.5 h-3.5 mt-0.5 text-red-400 shrink-0" />
      <div className="flex-1 min-w-0">
        <span className="text-[13px] font-medium text-red-300 block">
          Agent run failed
        </span>
        <span className="text-[12px] text-red-300/70 block mt-0.5 break-words">
          {errorMessage.slice(0, 200)}
          {errorMessage.length > 200 && "..."}
        </span>
      </div>
      {onDismiss !== undefined && (
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onDismiss}
          className="shrink-0 text-red-300/60 hover:text-red-300"
          aria-label="Dismiss error"
        >
          <X className="w-3.5 h-3.5" />
        </Button>
      )}
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export interface ChatMessagesProps {
  messages: Message[];
  isLoading: boolean;
  isSending: boolean;
  isAgentRunning: boolean;
  streamingToolCalls: ToolCall[];
  failedErrorMessage: string | undefined;
  onDismissError: (() => void) | undefined;
  messagesEndRef: RefObject<HTMLDivElement | null>;
  /** Resolved hook events (completed + blocks) */
  hookEvents?: HookEvent[];
  /** Currently running hooks */
  activeHooks?: HookStartedEvent[];
}

export function ChatMessages({
  messages,
  isLoading,
  isSending,
  isAgentRunning,
  streamingToolCalls,
  failedErrorMessage,
  onDismissError,
  messagesEndRef,
  hookEvents = [],
  activeHooks = [],
}: ChatMessagesProps) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const isTestEnv = import.meta.env.VITEST;

  // Compute streaming hash for auto-scroll hook
  const streamingHash = useMemo(() => {
    if (!isAgentRunning && !isSending) return undefined;
    return JSON.stringify({
      toolCalls: streamingToolCalls.map(tc => tc.id),
      activeHooks: activeHooks.map(h => h.hookName),
    });
  }, [isAgentRunning, isSending, streamingToolCalls, activeHooks]);

  // Use unified auto-scroll hook
  const { handleFollowOutput, handleAtBottomStateChange, isAtBottom, scrollToBottom } =
    useChatAutoScroll({
      messageCount: messages.length,
      isStreaming: isAgentRunning || isSending,
      streamingHash,
    });

  // Build merged timeline: messages + hook events sorted chronologically
  const timeline = useMemo(() => {
    const items: TimelineItem[] = [];

    // Add messages
    for (const msg of messages) {
      items.push({
        kind: "message",
        data: msg,
        sortTime: new Date(msg.createdAt).getTime(),
      });
    }

    // Add resolved hook events (completed + blocks)
    for (const ev of hookEvents) {
      items.push({
        kind: "hook",
        data: ev,
        sortTime: ev.timestamp,
      });
    }

    // Add active (running) hooks — they sort by their own timestamp
    for (const ev of activeHooks) {
      items.push({
        kind: "hook",
        data: ev,
        sortTime: ev.timestamp,
      });
    }

    // Sort chronologically
    items.sort((a, b) => a.sortTime - b.sortTime);

    return items;
  }, [messages, hookEvents, activeHooks]);

  const isEmpty = !isLoading && timeline.length === 0;

  if (isLoading) {
    return (
      <div className="flex-1 p-3" data-testid="chat-panel-messages">
        <LoadingState />
      </div>
    );
  }

  if (isEmpty) {
    return (
      <div className="flex-1 p-3" data-testid="chat-panel-messages">
        <EmptyState />
      </div>
    );
  }

  if (isTestEnv) {
    return (
      <div className="flex-1 overflow-hidden" data-testid="chat-panel-messages">
        <div className="px-3 pt-3">
          {failedErrorMessage && (
            <FailedRunBanner
              errorMessage={failedErrorMessage}
              onDismiss={onDismissError}
            />
          )}
        </div>
        {timeline.map((item, index) => (
          <div key={`${item.kind}-${item.sortTime}-${index}`} className="px-3">
            {item.kind === "hook" ? (
              <HookEventMessage event={item.data} />
            ) : (
              <MessageItem
                key={item.data.id}
                role={item.data.role}
                content={item.data.content}
                createdAt={item.data.createdAt}
                toolCalls={item.data.toolCalls ?? null}
                contentBlocks={item.data.contentBlocks ?? null}
              />
            )}
          </div>
        ))}
        <div className="px-3 pb-3">
          {(isSending || isAgentRunning) && (
            streamingToolCalls.length > 0 ? (
              <StreamingToolIndicator toolCalls={streamingToolCalls} isActive={true} />
            ) : (
              <TypingIndicator />
            )
          )}
          <div ref={messagesEndRef} />
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-hidden relative" data-testid="chat-panel-messages">
      <Virtuoso
        ref={virtuosoRef}
        data={timeline}
        followOutput={handleFollowOutput}
        atBottomStateChange={handleAtBottomStateChange}
        atBottomThreshold={150}
        alignToBottom
        className="h-full"
        components={{
          Header: () => (
            <div className="px-3 pt-3">
              {/* Show failed run banner if provided */}
              {failedErrorMessage && (
                <FailedRunBanner
                  errorMessage={failedErrorMessage}
                  onDismiss={onDismissError}
                />
              )}
            </div>
          ),
          Footer: () => (
            <div className="px-3 pb-3">
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
        itemContent={(_, item) => {
          if (item.kind === "hook") {
            return (
              <div className="px-3">
                <HookEventMessage event={item.data} />
              </div>
            );
          }
          const msg = item.data;
          return (
            <div className="px-3">
              <MessageItem
                key={msg.id}
                role={msg.role}
                content={msg.content}
                createdAt={msg.createdAt}
                toolCalls={msg.toolCalls ?? null}
                contentBlocks={msg.contentBlocks ?? null}
              />
            </div>
          );
        }}
      />
      {/* Scroll-to-bottom button */}
      {!isAtBottom && messages.length > 5 && (
        <Button
          size="icon-sm"
          variant="outline"
          onClick={scrollToBottom}
          className="absolute bottom-4 right-4 rounded-full shadow-lg"
          style={{
            background: "linear-gradient(135deg, rgba(255,107,53,0.95) 0%, rgba(255,107,53,0.85) 100%)",
            border: "1px solid rgba(255,107,53,0.3)",
          }}
          aria-label="Scroll to bottom"
        >
          <ArrowDown className="w-4 h-4 text-white" />
        </Button>
      )}
    </div>
  );
}

export { TypingIndicator, EmptyState, LoadingState, FailedRunBanner };
