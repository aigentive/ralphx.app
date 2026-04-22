/**
 * IntegratedChatPanel.components - Sub-components for IntegratedChatPanel
 */

import { Bot, MessageSquare, CheckSquare, FolderKanban, Hammer, Activity, X, History, GitMerge } from "lucide-react";
import { Button } from "@/components/ui/button";
import { withAlpha } from "@/lib/theme-colors";
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
        style={{ color: "var(--text-muted)" }}
      />
      <div
        className="px-3 py-2 rounded-lg"
        style={{
          /* macOS Tahoe: flat solid color, no gradient, no border */
          backgroundColor: "var(--bg-elevated)",
        }}
      >
        <div className="flex items-center gap-1">
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "var(--text-muted)" }}
          />
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "var(--text-muted)" }}
          />
          <div
            className="typing-dot w-1.5 h-1.5 rounded-full"
            style={{ backgroundColor: "var(--text-muted)" }}
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
          backgroundColor: "var(--bg-hover)",
        }}
      >
        <MessageSquare
          className="w-5 h-5"
          style={{ color: "var(--text-muted)" }}
        />
      </div>
      <p
        className="text-[13px] font-medium"
        style={{ color: "var(--text-primary)" }}
      >
        Start a conversation
      </p>
      <p
        className="text-xs mt-1"
        style={{ color: "var(--text-muted)" }}
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
          backgroundColor: "var(--bg-hover)",
        }}
      >
        <MessageSquare
          className="w-5 h-5"
          style={{ color: "var(--text-muted)" }}
        />
      </div>
      <p
        className="text-[13px] font-medium"
        style={{ color: "var(--text-primary)" }}
      >
        No chat for this state
      </p>
      <p
        className="text-xs mt-1"
        style={{ color: "var(--text-muted)" }}
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
        backgroundColor: "var(--status-error-muted)",
      }}
    >
      <Activity
        className="w-3.5 h-3.5 mt-0.5 shrink-0"
        style={{ color: "var(--status-error)" }}
      />
      <div className="flex-1 min-w-0">
        <span
          className="text-[13px] font-medium block"
          style={{ color: "var(--status-error)" }}
        >
          Agent run failed
        </span>
        <span
          className="text-[12px] block mt-0.5 break-words"
          style={{ color: "var(--status-error)" }}
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
          style={{ color: "var(--status-error)" }}
          aria-label="Dismiss error"
        >
          <X className="w-3.5 h-3.5" />
        </Button>
      )}
    </div>
  );
}

interface PreviousRunBannerProps {
  agentRunStatus: string | null;
  contextType: string;
}

export function PreviousRunBanner({ agentRunStatus, contextType }: PreviousRunBannerProps) {
  const contextLabel = contextType === "merge" ? "merge agent"
    : contextType === "review" ? "reviewer"
    : "worker";

  const statusLabel = agentRunStatus === "failed" ? "failed"
    : agentRunStatus === "cancelled" ? "cancelled"
    : agentRunStatus === "running" ? "in progress"
    : "completed";

  return (
    <div
      data-testid="previous-run-banner"
      className="px-3 py-1.5 flex items-center gap-2 shrink-0"
      style={{
        backgroundColor: withAlpha("var(--bg-elevated)", 80),
        borderBottom: "1px solid var(--overlay-weak)",
      }}
    >
      <History className="w-3 h-3" style={{ color: "var(--text-muted)" }} />
      <span className="text-[11px]" style={{ color: "var(--text-muted)" }}>
        Previous {contextLabel} run ({statusLabel})
      </span>
    </div>
  );
}

interface ContextIndicatorProps {
  context: ChatContext;
  isExecutionMode?: boolean;
  isReviewMode?: boolean;
  isMergeMode?: boolean;
}

export function ContextIndicator({
  context,
  isExecutionMode = false,
  isReviewMode = false,
  isMergeMode = false,
}: ContextIndicatorProps) {
  const getContextInfo = () => {
    if (isExecutionMode) {
      return { icon: Hammer, label: "Worker" };
    }
    if (isReviewMode) {
      return { icon: Bot, label: "AI Review" };
    }
    if (isMergeMode) {
      return { icon: GitMerge, label: "Merger" };
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
      default:
        return { icon: MessageSquare, label: "Chat" };
    }
  };

  const { icon: Icon, label } = getContextInfo();

  return (
    <div className="flex items-center gap-2 min-w-0 flex-1">
      <Icon
        className="w-3.5 h-3.5 shrink-0"
        style={{ color: "var(--text-muted)" }}
      />
      <span
        className="text-[13px] font-medium truncate"
        style={{ color: "var(--text-primary)" }}
      >
        {label}
      </span>
    </div>
  );
}
