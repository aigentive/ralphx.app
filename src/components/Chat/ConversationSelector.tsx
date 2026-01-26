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
import { formatDistanceToNow } from "date-fns";
import { History, Plus, Circle } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { ChatConversation, ContextType } from "@/types/chat-conversation";
import { cn } from "@/lib/utils";

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
function getConversationTitle(conversation: ChatConversation): string {
  if (conversation.title) {
    return conversation.title;
  }

  // Fallback title
  if (conversation.messageCount === 0) {
    return "New conversation";
  }

  return `Conversation ${conversation.id.slice(0, 8)}`;
}

// ============================================================================
// Component
// ============================================================================

export function ConversationSelector({
  conversations,
  activeConversationId,
  onSelectConversation,
  onNewConversation,
  isLoading = false,
}: ConversationSelectorProps) {
  // Sort conversations by last message date (most recent first)
  const sortedConversations = useMemo(() => {
    return [...conversations].sort((a, b) => {
      const aDate = a.lastMessageAt ? new Date(a.lastMessageAt).getTime() : 0;
      const bDate = b.lastMessageAt ? new Date(b.lastMessageAt).getTime() : 0;
      return bDate - aDate; // Descending
    });
  }, [conversations]);

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
          Conversation History
        </DropdownMenuLabel>

        {/* New Conversation Option */}
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
          sortedConversations.map((conversation) => {
            const isActive = conversation.id === activeConversationId;
            const title = getConversationTitle(conversation);
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
          })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
