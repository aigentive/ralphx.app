/**
 * ConversationSelector - Dropdown for switching between chat conversations
 *
 * Features:
 * - Lists all conversations for the current context (ideation/task/project)
 * - Shows conversation title, date, and message count
 * - Active conversation indicator (filled dot)
 * - "New Conversation" option at top
 * - Click to switch conversations
 */

import { useMemo } from "react";
import { formatDistanceToNow, format } from "date-fns";
import { History, Plus, Circle, CheckCircle2, XCircle, AlertCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ChatConversation, ContextType, AgentRunStatus } from "@/types/chat-conversation";
import { cn } from "@/lib/utils";
import { useQueries } from "@tanstack/react-query";
import { chatApi } from "@/api/chat";

// ============================================================================
// Types
// ============================================================================

export interface ConversationSelectorProps {
  /** Current context type */
  contextType: ContextType;
  /** Current context ID */
  contextId: string;
  /** All conversations for this context */
  conversations: ChatConversation[];
  /** ID of the active conversation */
  activeConversationId: string | null;
  /** Callback when a conversation is selected */
  onSelectConversation: (conversationId: string) => void;
  /** Callback when "New Conversation" is clicked */
  onNewConversation: () => void;
  /** Optional: show loading state */
  isLoading?: boolean;
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Format conversation date as relative time (e.g., "2 hours ago")
 * Falls back to "No messages" if no last message
 */
function formatConversationDate(lastMessageAt: string | null): string {
  if (!lastMessageAt) {
    return "No messages";
  }

  try {
    return formatDistanceToNow(new Date(lastMessageAt), { addSuffix: true });
  } catch {
    return "Unknown";
  }
}

/**
 * Generate a conversation title from first message or fallback
 */
function getConversationTitle(conversation: ChatConversation, index?: number): string {
  if (conversation.title) {
    return conversation.title;
  }

  // For task_execution context, show "Execution #N"
  if (conversation.contextType === "task_execution" && index !== undefined) {
    return `Execution #${index + 1}`;
  }

  // For review context, show "Review #N"
  if (conversation.contextType === "review" && index !== undefined) {
    return `Review #${index + 1}`;
  }

  // Fallback title
  if (conversation.messageCount === 0) {
    return "New conversation";
  }

  return `Conversation ${conversation.id.slice(0, 8)}`;
}

/**
 * Get status icon for execution conversations
 */
function getStatusIcon(status: AgentRunStatus | null) {
  switch (status) {
    case "running":
      return <Circle className="h-3 w-3 text-[var(--accent-primary)] fill-[var(--accent-primary)] animate-pulse" />;
    case "completed":
      return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case "failed":
      return <XCircle className="h-3 w-3 text-red-500" />;
    case "cancelled":
      return <AlertCircle className="h-3 w-3 text-yellow-500" />;
    default:
      return null;
  }
}

/**
 * Format execution date for display
 */
function formatExecutionDate(createdAt: string): string {
  try {
    return format(new Date(createdAt), "MMM d, h:mm a");
  } catch {
    return "Unknown";
  }
}

// ============================================================================
// Component
// ============================================================================

export function ConversationSelector({
  contextType,
  conversations,
  activeConversationId,
  onSelectConversation,
  onNewConversation,
  isLoading = false,
}: ConversationSelectorProps) {
  // Execution and review contexts share similar behavior (agent runs, status polling)
  const isAgentContext = contextType === "task_execution" || contextType === "review";
  const isExecutionContext = contextType === "task_execution";

  // Sort conversations by creation date for agent contexts, last message date otherwise
  const sortedConversations = useMemo(() => {
    return [...conversations].sort((a, b) => {
      if (isAgentContext) {
        // For agent contexts (execution/review), sort by creation date DESC (most recent first)
        const aDate = new Date(a.createdAt).getTime();
        const bDate = new Date(b.createdAt).getTime();
        return bDate - aDate;
      } else {
        // For regular conversations, sort by last message date
        const aDate = a.lastMessageAt ? new Date(a.lastMessageAt).getTime() : 0;
        const bDate = b.lastMessageAt ? new Date(b.lastMessageAt).getTime() : 0;
        return bDate - aDate;
      }
    });
  }, [conversations, isAgentContext]);

  // Fetch agent run status for all agent context conversations using useQueries
  // This enables status polling for both execution and review contexts
  const statusQueries = useQueries({
    queries: sortedConversations.map((conv) => ({
      queryKey: ["agent-run", conv.id] as const,
      queryFn: () => chatApi.getAgentRunStatus(conv.id),
      enabled: isAgentContext,
      // Poll every 2 seconds for running agents
      refetchInterval: 2000,
      // Only refetch if we're in agent context
      refetchIntervalInBackground: false,
    })),
  });

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-hover)] transition-colors"
          aria-label="Conversation history"
          data-testid="conversation-selector-trigger"
        >
          <History className="h-4 w-4" />
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent
        align="end"
        className="w-[320px] bg-[var(--bg-elevated)] border border-[var(--border-default)] rounded-lg shadow-lg"
        data-testid="conversation-selector-menu"
      >
        <DropdownMenuLabel className="text-[var(--text-secondary)] text-xs font-medium tracking-wide uppercase px-3 py-2">
          {isExecutionContext ? "Execution History" : contextType === "review" ? "Review History" : "Conversation History"}
        </DropdownMenuLabel>

        {/* New Conversation Option - Only for non-agent contexts (not execution or review) */}
        {!isAgentContext && (
          <>
            <DropdownMenuItem
              onClick={onNewConversation}
              className={cn(
                "flex items-center gap-3 px-3 py-2.5 cursor-pointer",
                "text-[var(--text-primary)] hover:bg-[var(--bg-hover)]",
                "transition-colors"
              )}
              data-testid="conversation-selector-new"
            >
              <Plus className="h-4 w-4 text-[var(--accent-primary)]" />
              <span className="text-sm font-medium">New Conversation</span>
            </DropdownMenuItem>

            {/* Separator */}
            {sortedConversations.length > 0 && (
              <DropdownMenuSeparator className="bg-[var(--border-subtle)] my-1" />
            )}
          </>
        )}

        {/* Loading State */}
        {isLoading && (
          <div className="px-3 py-6 text-center text-[var(--text-muted)] text-sm">
            Loading conversations...
          </div>
        )}

        {/* Empty State */}
        {!isLoading && sortedConversations.length === 0 && (
          <div className="px-3 py-6 text-center text-[var(--text-muted)] text-sm">
            No conversations yet
          </div>
        )}

        {/* Conversation List */}
        {!isLoading &&
          sortedConversations.map((conversation, index) => {
            const isActive = conversation.id === activeConversationId;
            const title = getConversationTitle(conversation, index);

            // Get agent run status for agent context conversations (execution/review)
            const agentRunStatus = isAgentContext && statusQueries[index]
              ? statusQueries[index].data?.status ?? null
              : null;

            if (isAgentContext) {
              // Agent context rendering (execution and review)
              const executionDate = formatExecutionDate(conversation.createdAt);
              const statusIcon = getStatusIcon(agentRunStatus);

              return (
                <DropdownMenuItem
                  key={conversation.id}
                  onClick={() => onSelectConversation(conversation.id)}
                  className={cn(
                    "flex items-start gap-3 px-3 py-2.5 cursor-pointer",
                    "hover:bg-[var(--bg-hover)] transition-colors",
                    isActive && "bg-[var(--accent-muted)]"
                  )}
                  data-testid={`conversation-item-${conversation.id}`}
                  data-active={isActive}
                >
                  {/* Status Icon */}
                  <div className="mt-1.5 flex-shrink-0">
                    {statusIcon || (
                      <Circle
                        className={cn(
                          "h-2 w-2",
                          isActive
                            ? "fill-[var(--accent-primary)] text-[var(--accent-primary)]"
                            : "text-transparent"
                        )}
                      />
                    )}
                  </div>

                  {/* Content */}
                  <div className="flex-1 min-w-0">
                    {/* Title with status */}
                    <div className="flex items-center gap-2">
                      <div
                        className={cn(
                          "text-sm font-medium truncate",
                          isActive
                            ? "text-[var(--text-primary)]"
                            : "text-[var(--text-secondary)]"
                        )}
                      >
                        {title}
                      </div>
                      {agentRunStatus === "running" && (
                        <span className="text-[10px] text-[var(--accent-primary)] font-medium uppercase tracking-wide">
                          Running
                        </span>
                      )}
                    </div>

                    {/* Date and status */}
                    <div className="flex items-center gap-2 mt-0.5 text-xs text-[var(--text-muted)]">
                      <span>{executionDate}</span>
                      {agentRunStatus && agentRunStatus !== "running" && (
                        <>
                          <span>•</span>
                          <span className={cn(
                            agentRunStatus === "completed" && "text-green-500",
                            agentRunStatus === "failed" && "text-red-500",
                            agentRunStatus === "cancelled" && "text-yellow-500"
                          )}>
                            {agentRunStatus.charAt(0).toUpperCase() + agentRunStatus.slice(1)}
                          </span>
                        </>
                      )}
                    </div>
                  </div>
                </DropdownMenuItem>
              );
            } else {
              // Regular conversation rendering
              const date = formatConversationDate(conversation.lastMessageAt);

              return (
                <DropdownMenuItem
                  key={conversation.id}
                  onClick={() => onSelectConversation(conversation.id)}
                  className={cn(
                    "flex items-start gap-3 px-3 py-2.5 cursor-pointer",
                    "hover:bg-[var(--bg-hover)] transition-colors",
                    isActive && "bg-[var(--accent-muted)]"
                  )}
                  data-testid={`conversation-item-${conversation.id}`}
                  data-active={isActive}
                >
                  {/* Active Indicator */}
                  <Circle
                    className={cn(
                      "h-2 w-2 mt-1.5 flex-shrink-0",
                      isActive
                        ? "fill-[var(--accent-primary)] text-[var(--accent-primary)]"
                        : "text-transparent"
                    )}
                  />

                  {/* Content */}
                  <div className="flex-1 min-w-0">
                    {/* Title */}
                    <div
                      className={cn(
                        "text-sm font-medium truncate",
                        isActive
                          ? "text-[var(--text-primary)]"
                          : "text-[var(--text-secondary)]"
                      )}
                    >
                      {title}
                    </div>

                    {/* Date and Message Count */}
                    <div className="flex items-center gap-2 mt-0.5 text-xs text-[var(--text-muted)]">
                      <span>{date}</span>
                      <span>•</span>
                      <span>
                        {conversation.messageCount}{" "}
                        {conversation.messageCount === 1 ? "message" : "messages"}
                      </span>
                    </div>
                  </div>
                </DropdownMenuItem>
              );
            }
          })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
