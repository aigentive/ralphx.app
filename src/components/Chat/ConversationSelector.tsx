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

  // For merge context, show "Merge #N"
  if (conversation.contextType === "merge" && index !== undefined) {
    return `Merge #${index + 1}`;
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
      return <Circle className="h-3 w-3 animate-pulse" style={{ color: "hsl(14 100% 60%)", fill: "hsl(14 100% 60%)" }} />;
    case "completed":
      return <CheckCircle2 className="h-3 w-3" style={{ color: "hsl(145 60% 50%)" }} />;
    case "failed":
      return <XCircle className="h-3 w-3" style={{ color: "hsl(0 70% 60%)" }} />;
    case "cancelled":
      return <AlertCircle className="h-3 w-3" style={{ color: "hsl(45 90% 55%)" }} />;
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
  // Execution, review, and merge contexts share similar behavior (agent runs, status polling)
  const isAgentContext = contextType === "task_execution" || contextType === "review" || contextType === "merge";
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
        className="w-[320px] rounded-xl overflow-y-auto"
        style={{
          /* macOS Tahoe: flat dark background, subtle border */
          backgroundColor: "hsl(220 10% 14%)",
          border: "1px solid hsla(220 10% 100% / 0.08)",
          boxShadow: "0 8px 32px hsla(0 0% 0% / 0.4)",
          maxHeight: "400px",
        }}
        data-testid="conversation-selector-menu"
      >
        <DropdownMenuLabel
          className="text-[11px] font-medium tracking-wide uppercase px-3 py-2"
          style={{ color: "hsl(220 10% 50%)" }}
        >
          {isExecutionContext ? "Execution History" : contextType === "review" ? "Review History" : contextType === "merge" ? "Merge History" : "Conversation History"}
        </DropdownMenuLabel>

        {/* New Conversation Option - Only for non-agent contexts (not execution or review) */}
        {!isAgentContext && (
          <>
            <DropdownMenuItem
              onClick={onNewConversation}
              className="flex items-center gap-3 px-3 py-2.5 cursor-pointer transition-colors"
              style={{ color: "hsl(220 10% 90%)" }}
              data-testid="conversation-selector-new"
            >
              <Plus className="h-4 w-4" style={{ color: "hsl(14 100% 60%)" }} />
              <span className="text-[13px] font-medium">New Conversation</span>
            </DropdownMenuItem>

            {/* Separator */}
            {sortedConversations.length > 0 && (
              <DropdownMenuSeparator style={{ backgroundColor: "hsla(220 10% 100% / 0.06)", margin: "4px 0" }} />
            )}
          </>
        )}

        {/* Loading State */}
        {isLoading && (
          <div className="px-3 py-6 text-center text-[13px]" style={{ color: "hsl(220 10% 50%)" }}>
            Loading conversations...
          </div>
        )}

        {/* Empty State */}
        {!isLoading && sortedConversations.length === 0 && (
          <div className="px-3 py-6 text-center text-[13px]" style={{ color: "hsl(220 10% 50%)" }}>
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
                  className="flex items-start gap-3 px-3 py-2.5 cursor-pointer transition-colors rounded-lg mx-1"
                  style={{
                    backgroundColor: isActive ? "hsla(14 100% 60% / 0.15)" : "transparent",
                  }}
                  data-testid={`conversation-item-${conversation.id}`}
                  data-active={isActive}
                >
                  {/* Status Icon */}
                  <div className="mt-1.5 flex-shrink-0">
                    {statusIcon || (
                      <Circle
                        className="h-2 w-2"
                        style={{
                          color: isActive ? "hsl(14 100% 60%)" : "transparent",
                          fill: isActive ? "hsl(14 100% 60%)" : "transparent",
                        }}
                      />
                    )}
                  </div>

                  {/* Content */}
                  <div className="flex-1 min-w-0">
                    {/* Title with status */}
                    <div className="flex items-center gap-2">
                      <div
                        className="text-[13px] font-medium truncate"
                        style={{ color: isActive ? "hsl(220 10% 95%)" : "hsl(220 10% 75%)" }}
                      >
                        {title}
                      </div>
                      {agentRunStatus === "running" && (
                        <span
                          className="text-[10px] font-medium uppercase tracking-wide"
                          style={{ color: "hsl(14 100% 60%)" }}
                        >
                          Running
                        </span>
                      )}
                    </div>

                    {/* Date and status */}
                    <div
                      className="flex items-center gap-2 mt-0.5 text-[11px]"
                      style={{ color: "hsl(220 10% 50%)" }}
                    >
                      <span>{executionDate}</span>
                      {agentRunStatus && agentRunStatus !== "running" && (
                        <>
                          <span>•</span>
                          <span style={{
                            color: agentRunStatus === "completed" ? "hsl(145 60% 50%)"
                              : agentRunStatus === "failed" ? "hsl(0 70% 60%)"
                              : agentRunStatus === "cancelled" ? "hsl(45 90% 55%)"
                              : undefined
                          }}>
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
                  className="flex items-start gap-3 px-3 py-2.5 cursor-pointer transition-colors rounded-lg mx-1"
                  style={{
                    backgroundColor: isActive ? "hsla(14 100% 60% / 0.15)" : "transparent",
                  }}
                  data-testid={`conversation-item-${conversation.id}`}
                  data-active={isActive}
                >
                  {/* Active Indicator */}
                  <Circle
                    className="h-2 w-2 mt-1.5 flex-shrink-0"
                    style={{
                      color: isActive ? "hsl(14 100% 60%)" : "transparent",
                      fill: isActive ? "hsl(14 100% 60%)" : "transparent",
                    }}
                  />

                  {/* Content */}
                  <div className="flex-1 min-w-0">
                    {/* Title */}
                    <div
                      className="text-[13px] font-medium truncate"
                      style={{ color: isActive ? "hsl(220 10% 95%)" : "hsl(220 10% 75%)" }}
                    >
                      {title}
                    </div>

                    {/* Date and Message Count */}
                    <div
                      className="flex items-center gap-2 mt-0.5 text-[11px]"
                      style={{ color: "hsl(220 10% 50%)" }}
                    >
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
