/**
 * ChatMessages - Message rendering and display logic
 */

import { useMemo, type RefObject } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { MessageItem } from "./MessageItem";
import { StreamingToolIndicator } from "./StreamingToolIndicator";
import { type ToolCall } from "./ToolCallIndicator";
import { type Message } from "@/types/chat";
import { Bot, MessageSquare, Loader2, Activity, Hammer, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useUiStore } from "@/stores/uiStore";

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

function WorkerExecutingIndicator() {
  const setCurrentView = useUiStore((s) => s.setCurrentView);

  return (
    <div
      data-testid="worker-executing-indicator"
      className="flex items-center gap-2 px-3 py-2 mb-2 rounded-lg"
      style={{
        background: "linear-gradient(135deg, rgba(255,107,53,0.08) 0%, rgba(255,107,53,0.03) 100%)",
        border: "1px solid rgba(255,107,53,0.15)",
      }}
    >
      <Hammer className="w-3.5 h-3.5 text-[#ff6b35]" />
      <div className="flex items-center gap-2 flex-1">
        <span className="text-[13px] font-medium text-white/80">Worker is executing...</span>
        <div className="flex items-center gap-1">
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-[#ff6b35]" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-[#ff6b35]" />
          <div className="typing-dot w-1.5 h-1.5 rounded-full bg-[#ff6b35]" />
        </div>
      </div>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setCurrentView("activity")}
        className="shrink-0 h-7 px-2"
        aria-label="View all activity"
      >
        <Activity className="w-3.5 h-3.5 mr-1" />
        <span className="text-[11px]">Activity</span>
      </Button>
    </div>
  );
}

interface FailedRunBannerProps {
  errorMessage: string;
  onDismiss?: () => void;
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
      {onDismiss && (
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
  isExecutionMode: boolean;
  streamingToolCalls: ToolCall[];
  failedErrorMessage?: string;
  onDismissError?: () => void;
  messagesEndRef: RefObject<HTMLDivElement>;
  scrollAreaRef: RefObject<HTMLDivElement>;
}

export function ChatMessages({
  messages,
  isLoading,
  isSending,
  isAgentRunning,
  isExecutionMode,
  streamingToolCalls,
  failedErrorMessage,
  onDismissError,
  messagesEndRef,
  scrollAreaRef,
}: ChatMessagesProps) {
  // Sort messages by createdAt - render in chronological order
  const sortedMessages = useMemo(() => {
    return [...messages].sort((a, b) =>
      new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
    );
  }, [messages]);

  const isEmpty = !isLoading && sortedMessages.length === 0;

  return (
    <ScrollArea
      ref={scrollAreaRef}
      className="flex-1"
      data-testid="chat-panel-messages"
    >
      <div className="p-3">
        {/* Show failed run banner if provided */}
        {failedErrorMessage && (
          <FailedRunBanner
            errorMessage={failedErrorMessage}
            onDismiss={onDismissError}
          />
        )}

        {/* Show worker executing indicator when in execution mode */}
        {isExecutionMode && <WorkerExecutingIndicator />}

        {isLoading ? (
          <LoadingState />
        ) : isEmpty ? (
          <EmptyState />
        ) : (
          <>
            {sortedMessages.map((msg) => (
              <MessageItem
                key={msg.id}
                role={msg.role}
                content={msg.content}
                createdAt={msg.createdAt}
                toolCalls={msg.toolCalls}
                contentBlocks={msg.contentBlocks}
              />
            ))}
            {/* Show streaming tool calls or typing indicator while agent is working */}
            {(isSending || isAgentRunning) && (
              streamingToolCalls.length > 0 ? (
                <StreamingToolIndicator toolCalls={streamingToolCalls} isActive={true} />
              ) : (
                <TypingIndicator />
              )
            )}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>
    </ScrollArea>
  );
}

export { TypingIndicator, EmptyState, LoadingState, WorkerExecutingIndicator, FailedRunBanner };
