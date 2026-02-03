/**
 * IntegratedChatPanel.components - Sub-components for IntegratedChatPanel
 */

import { Bot, MessageSquare, CheckSquare, FolderKanban, Hammer, Activity, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ChatContext } from "@/types/chat";

// ============================================================================
// CSS Animations
// ============================================================================

export const animationStyles = `
@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

.typing-dot {
  animation: typingBounce 1.4s ease-in-out infinite;
}

.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }
`;

// ============================================================================
// Sub-components
// ============================================================================

export function TypingIndicator() {
  return (
    <div
      data-testid="chat-typing-indicator"
      className="flex items-start gap-2 mb-2"
    >
      <Bot
        className="w-3.5 h-3.5 mt-2 shrink-0"
        style={{ color: "hsl(220 10% 45%)" }}
      />
      <div
        className="px-3 py-2 rounded-lg"
        style={{
          /* macOS Tahoe: flat solid color, no gradient, no border */
          backgroundColor: "hsl(220 10% 14%)",
        }}
      >
        <div className="flex items-center gap-1">
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "hsl(220 10% 40%)" }}
          />
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "hsl(220 10% 40%)" }}
          />
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "hsl(220 10% 40%)" }}
          />
        </div>
      </div>
    </div>
  );
}

export function EmptyState() {
  return (
    <div
      data-testid="chat-panel-empty"
      className="flex flex-col items-center justify-center h-full p-6 text-center"
    >
      <div
        className="w-12 h-12 rounded-lg flex items-center justify-center mb-3"
        style={{
          /* macOS Tahoe: subtle solid background, no gradient, no border */
          backgroundColor: "hsl(220 10% 16%)",
        }}
      >
        <MessageSquare
          className="w-5 h-5"
          style={{ color: "hsl(220 10% 50%)" }}
        />
      </div>
      <p
        className="text-[13px] font-medium"
        style={{ color: "hsl(220 10% 85%)" }}
      >
        Start a conversation
      </p>
      <p
        className="text-xs mt-1"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        Ask questions or get help with your tasks
      </p>
    </div>
  );
}

export function HistoryEmptyState() {
  return (
    <div
      data-testid="chat-panel-history-empty"
      className="flex flex-col items-center justify-center h-full p-6 text-center"
    >
      <div
        className="w-12 h-12 rounded-lg flex items-center justify-center mb-3"
        style={{
          backgroundColor: "hsl(220 10% 16%)",
        }}
      >
        <MessageSquare
          className="w-5 h-5"
          style={{ color: "hsl(220 10% 50%)" }}
        />
      </div>
      <p
        className="text-[13px] font-medium"
        style={{ color: "hsl(220 10% 85%)" }}
      >
        No chat for this state
      </p>
      <p
        className="text-xs mt-1"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        This historical state does not have a conversation attached.
      </p>
    </div>
  );
}

// LoadingState is now extracted to MessageListSkeleton.tsx
// Re-export for backwards compatibility
export { MessageListSkeleton as LoadingState } from "./MessageListSkeleton";

interface FailedRunBannerProps {
  errorMessage: string;
  onDismiss?: () => void;
}

export function FailedRunBanner({ errorMessage, onDismiss }: FailedRunBannerProps) {
  return (
    <div
      data-testid="failed-run-banner"
      className="flex items-start gap-2 px-3 py-2 mb-2 rounded-lg"
      style={{
        /* macOS Tahoe: subtle solid background with error tint, no gradient, no border */
        backgroundColor: "hsla(0 70% 55% / 0.12)",
      }}
    >
      <Activity
        className="w-3.5 h-3.5 mt-0.5 shrink-0"
        style={{ color: "hsl(0 70% 60%)" }}
      />
      <div className="flex-1 min-w-0">
        <span
          className="text-[13px] font-medium block"
          style={{ color: "hsl(0 70% 70%)" }}
        >
          Agent run failed
        </span>
        <span
          className="text-[12px] block mt-0.5 break-words"
          style={{ color: "hsl(0 70% 60%)" }}
        >
          {errorMessage.slice(0, 200)}
          {errorMessage.length > 200 && "..."}
        </span>
      </div>
      {onDismiss && (
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onDismiss}
          className="shrink-0"
          style={{ color: "hsl(0 70% 60%)" }}
          aria-label="Dismiss error"
        >
          <X className="w-3.5 h-3.5" />
        </Button>
      )}
    </div>
  );
}

interface ContextIndicatorProps {
  context: ChatContext;
  isExecutionMode?: boolean;
  isReviewMode?: boolean;
}

export function ContextIndicator({ context, isExecutionMode = false, isReviewMode = false }: ContextIndicatorProps) {
  const getContextInfo = () => {
    if (isExecutionMode) {
      return { icon: Hammer, label: "Worker Execution" };
    }
    if (isReviewMode) {
      return { icon: Bot, label: "AI Review" };
    }

    switch (context.view) {
      case "ideation":
        return { icon: MessageSquare, label: "Chat" };
      case "kanban":
        return context.selectedTaskId
          ? { icon: CheckSquare, label: "Task" }
          : { icon: FolderKanban, label: "Project" };
      case "task_detail":
        return { icon: CheckSquare, label: "Task" };
      case "activity":
        return { icon: MessageSquare, label: "Activity" };
      case "settings":
        return { icon: MessageSquare, label: "Settings" };
      default:
        return { icon: MessageSquare, label: "Chat" };
    }
  };

  const { icon: Icon, label } = getContextInfo();

  return (
    <div className="flex items-center gap-2 min-w-0 flex-1">
      <Icon
        className="w-3.5 h-3.5 shrink-0"
        style={{ color: "hsl(220 10% 50%)" }}
      />
      <span
        className="text-[13px] font-medium truncate"
        style={{ color: "hsl(220 10% 85%)" }}
      >
        {label}
      </span>
    </div>
  );
}
