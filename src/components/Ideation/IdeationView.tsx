/**
 * IdeationView - Premium two-panel ideation interface
 *
 * Features:
 * - Two-panel resizable layout with drag handle
 * - Conversation panel with styled message bubbles
 * - Typing indicator with animated dots
 * - Proposals panel with ProposalList
 * - Apply dropdown for target column selection
 * - Warm radial gradient background
 * - Glass effect headers
 *
 * Design spec: specs/design/pages/ideation-view.md
 */

import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import {
  MessageSquare,
  ListTodo,
  Plus,
  Archive,
  Lightbulb,
  MessageSquareText,
  Loader2,
  ChevronDown,
  FileEdit,
  Inbox,
  CheckSquare,
  Square,
  ArrowUpDown,
  Trash2,
  AlertCircle,
  Undo2,
  Eye,
  Upload,
  Bot,
  Clock,
  History,
} from "lucide-react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  IdeationSession,
  TaskProposal,
  ChatMessage as ChatMessageType,
  ApplyProposalsInput,
} from "@/types/ideation";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Checkbox } from "@/components/ui/checkbox";
import { Card } from "@/components/ui/card";
import ReactMarkdown from "react-markdown";
import type { Priority } from "@/types/ideation";
import { PlanDisplay } from "./PlanDisplay";
import { PlanHistoryDialog } from "./PlanHistoryDialog";
import { useIdeationStore, type ProactiveSyncNotification } from "@/stores/ideationStore";
import { ChatInput } from "@/components/Chat/ChatInput";
import { ToolCallIndicator, type ToolCall } from "@/components/Chat/ToolCallIndicator";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

interface IdeationViewProps {
  session: IdeationSession | null;
  /** All active sessions for the project (for session browser) */
  sessions: IdeationSession[];
  messages: ChatMessageType[];
  proposals: TaskProposal[];
  onSendMessage: (content: string) => void;
  onNewSession: () => void;
  onSelectSession: (sessionId: string) => void;
  onArchiveSession: (sessionId: string) => void;
  onSelectProposal: (proposalId: string) => void;
  onEditProposal: (proposalId: string) => void;
  onRemoveProposal: (proposalId: string) => void;
  onReorderProposals: (proposalIds: string[]) => void;
  onApply: (options: ApplyProposalsInput) => void;
  isLoading?: boolean;
}

// ============================================================================
// Priority Configuration
// ============================================================================

const PRIORITY_STYLES: Record<Priority, { bg: string; text: string }> = {
  critical: { bg: "bg-destructive", text: "text-destructive-foreground" },
  high: { bg: "bg-[#ff6b35]", text: "text-white" },
  medium: { bg: "bg-[rgba(255,107,53,0.2)]", text: "text-[#ff6b35]" },
  low: { bg: "bg-secondary", text: "text-secondary-foreground" },
};

// ============================================================================
// Markdown Components
// ============================================================================

const markdownComponents = {
  a: ({ href, children, ...props }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="underline hover:no-underline text-[var(--accent-primary)]"
      {...props}
    >
      {children}
    </a>
  ),
  code: ({ className, children, ...props }: React.HTMLAttributes<HTMLElement>) => {
    const isBlock = className?.includes("language-");
    if (isBlock) {
      return (
        <code
          className={`block p-3 rounded text-sm overflow-x-auto bg-[var(--bg-elevated)] ${className || ""}`}
          {...props}
        >
          {children}
        </code>
      );
    }
    return (
      <code className="px-1 py-0.5 rounded text-sm bg-[var(--bg-elevated)]" {...props}>
        {children}
      </code>
    );
  },
  pre: ({ children, ...props }: React.HTMLAttributes<HTMLPreElement>) => (
    <pre className="my-2 rounded overflow-hidden bg-[var(--bg-elevated)]" {...props}>
      {children}
    </pre>
  ),
  p: ({ children, ...props }: React.HTMLAttributes<HTMLParagraphElement>) => (
    <p className="mb-2 last:mb-0" {...props}>
      {children}
    </p>
  ),
  ul: ({ children, ...props }: React.HTMLAttributes<HTMLUListElement>) => (
    <ul className="list-disc list-inside mb-2" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }: React.HTMLAttributes<HTMLOListElement>) => (
    <ol className="list-decimal list-inside mb-2" {...props}>
      {children}
    </ol>
  ),
  li: ({ children, ...props }: React.LiHTMLAttributes<HTMLLIElement>) => (
    <li className="mb-1" {...props}>
      {children}
    </li>
  ),
};

// ============================================================================
// Typing Indicator
// ============================================================================

const animationStyles = `
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

function TypingIndicator() {
  return (
    <div
      data-testid="typing-indicator"
      className="flex items-start gap-2 mb-2"
    >
      <Bot className="w-3.5 h-3.5 mt-2.5 shrink-0 text-[var(--text-muted)]" />
      <div
        className="px-3.5 py-2.5 rounded-[10px_10px_10px_4px]"
        style={{
          backgroundColor: "var(--bg-elevated)",
          border: "1px solid var(--border-subtle)",
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

// ============================================================================
// Chat Message Bubble
// ============================================================================

interface MessageItemProps {
  role: string;
  content: string;
  createdAt: string;
  toolCalls?: string | null;
  isFirstInGroup?: boolean;
  isLastInGroup?: boolean;
}

function MessageItem({
  role,
  content,
  createdAt,
  toolCalls,
  isFirstInGroup = true,
  isLastInGroup = true,
}: MessageItemProps) {
  const isUser = role === "user";

  const timestamp = useMemo(() => {
    const date = new Date(createdAt);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;

    return date.toLocaleTimeString([], {
      hour: "numeric",
      minute: "2-digit",
    });
  }, [createdAt]);

  // Parse tool calls from JSON string
  const parsedToolCalls = useMemo((): ToolCall[] => {
    if (!toolCalls) return [];
    try {
      const parsed = JSON.parse(toolCalls);
      if (Array.isArray(parsed)) {
        return parsed.map((tc, idx) => ({
          id: tc.id ?? `tool-${idx}`,
          name: tc.name ?? "unknown",
          arguments: tc.arguments ?? {},
          result: tc.result,
          error: tc.error,
        }));
      }
      return [];
    } catch {
      return [];
    }
  }, [toolCalls]);

  return (
    <div
      className={cn(
        "flex",
        isUser ? "justify-end" : "justify-start",
        isLastInGroup ? "mb-3" : "mb-1"
      )}
    >
      {/* Agent indicator for first assistant message */}
      {!isUser && isFirstInGroup && (
        <Bot className="w-3.5 h-3.5 mt-2.5 mr-2 shrink-0 text-[var(--text-muted)]" />
      )}
      {!isUser && !isFirstInGroup && <div className="w-3.5 mr-2 shrink-0" />}

      <div className="flex flex-col max-w-[85%]">
        {/* Tool calls (shown before text content for assistant messages) */}
        {!isUser && parsedToolCalls.length > 0 && (
          <div className="space-y-1.5 mb-2">
            {parsedToolCalls.map((tc) => (
              <ToolCallIndicator key={tc.id} toolCall={tc} />
            ))}
          </div>
        )}

        {/* Message content */}
        <div
          className={cn(
            "px-3 py-2 text-sm",
            isUser
              ? "rounded-[10px_10px_4px_10px]"
              : "rounded-[10px_10px_10px_4px]"
          )}
          style={{
            backgroundColor: isUser
              ? "var(--accent-primary)"
              : "var(--bg-elevated)",
            color: isUser ? "white" : "var(--text-primary)",
            border: isUser ? "none" : "1px solid var(--border-subtle)",
            boxShadow: isUser ? "var(--shadow-xs)" : "none",
          }}
        >
          {isUser ? (
            <p className="whitespace-pre-wrap break-words">{content}</p>
          ) : (
            <div className="prose prose-sm prose-invert max-w-none">
              <ReactMarkdown components={markdownComponents}>
                {content}
              </ReactMarkdown>
            </div>
          )}
        </div>
        {isLastInGroup && (
          <span
            className={cn(
              "text-[11px] mt-1 px-1",
              isUser ? "text-right" : "text-left"
            )}
            style={{ color: "var(--text-muted)" }}
          >
            {timestamp}
          </span>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Empty States
// ============================================================================

function ConversationEmptyState() {
  return (
    <div className="flex flex-col items-center justify-center h-full p-8 text-center">
      <div className="p-6 rounded-lg border-2 border-dashed border-[var(--border-subtle)]">
        <MessageSquareText className="w-12 h-12 mx-auto mb-4 text-[var(--text-muted)]" />
        <p className="font-medium text-[var(--text-secondary)]">Start the conversation</p>
        <p className="text-sm text-[var(--text-muted)] mt-1">
          Describe your ideas and I'll help create task proposals
        </p>
      </div>
    </div>
  );
}

function ProposalsEmptyState() {
  return (
    <div
      data-testid="proposals-empty-state"
      className="flex flex-col items-center justify-center h-full p-12 text-center"
    >
      <div className="p-6 rounded-lg border-2 border-dashed border-[var(--border-subtle)]">
        <Lightbulb className="w-12 h-12 mx-auto mb-4 text-[var(--text-muted)]" />
        <p className="font-medium text-[var(--text-secondary)]">No proposals yet</p>
        <p className="text-sm text-[var(--text-muted)] mt-1">
          Chat with the orchestrator to generate task proposals
        </p>
      </div>
    </div>
  );
}

// ============================================================================
// Session Browser (Left Sidebar)
// ============================================================================

interface SessionBrowserProps {
  sessions: IdeationSession[];
  currentSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onNewSession: () => void;
}

function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function SessionBrowser({
  sessions,
  currentSessionId,
  onSelectSession,
  onNewSession,
}: SessionBrowserProps) {
  // Sort sessions by updatedAt descending (most recent first)
  const sortedSessions = useMemo(
    () => [...sessions].sort((a, b) =>
      new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
    ),
    [sessions]
  );

  return (
    <div
      data-testid="session-browser"
      className="flex flex-col h-full border-r border-[var(--border-subtle)] bg-[var(--bg-surface)]"
      style={{ width: "280px", minWidth: "280px", flexShrink: 0 }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[var(--border-subtle)]">
        <div className="flex items-center gap-2">
          <History className="w-4 h-4 text-[var(--text-secondary)]" />
          <h2 className="text-sm font-semibold text-[var(--text-primary)]">Sessions</h2>
          <Badge variant="secondary" className="text-xs">{sessions.length}</Badge>
        </div>
        <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onNewSession} title="New Session">
          <Plus className="w-4 h-4" />
        </Button>
      </div>

      {/* Session List */}
      <div className="flex-1 overflow-y-auto">
        {sortedSessions.length === 0 ? (
          <div className="p-4 text-center text-sm text-[var(--text-muted)]">
            No sessions yet
          </div>
        ) : (
          sortedSessions.map((session) => {
            const isSelected = session.id === currentSessionId;
            return (
              <div
                key={session.id}
                data-testid={`session-item-${session.id}`}
                className={cn(
                  "group px-4 py-3 border-b border-[var(--border-subtle)] transition-colors",
                  "hover:bg-[var(--bg-hover)]",
                  isSelected && "bg-[rgba(255,107,53,0.08)] border-l-2 border-l-[var(--accent-primary)]"
                )}
              >
                <div className="flex items-start justify-between gap-2">
                  <div className="flex-1 min-w-0">
                    <span className="text-sm font-medium text-[var(--text-primary)] truncate block">
                      {session.title ?? "Untitled Session"}
                    </span>
                    <div className="flex items-center gap-1 mt-1">
                      <Clock className="w-3 h-3 text-[var(--text-muted)]" />
                      <span className="text-xs text-[var(--text-muted)]">
                        {formatRelativeTime(session.updatedAt)}
                      </span>
                    </div>
                  </div>
                  <span
                    className="w-2 h-2 rounded-full flex-shrink-0 mt-1.5"
                    style={{ backgroundColor: "var(--status-success)" }}
                    title="Active"
                  />
                </div>
                {/* Continue button */}
                <Button
                  variant={isSelected ? "default" : "outline"}
                  size="sm"
                  className="w-full mt-2 h-7 text-xs"
                  onClick={() => onSelectSession(session.id)}
                >
                  {isSelected ? "Current Session" : "Continue"}
                </Button>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Start Session Panel (Right side when no session selected)
// ============================================================================

function StartSessionPanel({ onNewSession }: { onNewSession: () => void }) {
  return (
    <div className="flex-1 flex flex-col items-center justify-center p-8">
      <div className="p-8 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-center shadow-[var(--shadow-sm)] max-w-md">
        <Lightbulb className="w-12 h-12 mx-auto mb-4 text-[var(--accent-primary)]" />
        <h2 className="text-xl font-semibold mb-2 text-[var(--text-primary)] tracking-tight">
          Ideation
        </h2>
        <p className="text-sm mb-6 text-[var(--text-secondary)]">
          Select a session from the sidebar to continue, or start a new brainstorming session.
        </p>
        <Button onClick={onNewSession} className="px-6">
          <Plus className="w-4 h-4 mr-2" />
          New Session
        </Button>
      </div>
    </div>
  );
}


// ============================================================================
// Proposal Card (Premium)
// ============================================================================

interface ProposalCardProps {
  proposal: TaskProposal;
  onSelect: (proposalId: string) => void;
  onEdit: (proposalId: string) => void;
  onRemove: (proposalId: string) => void;
  isHighlighted?: boolean;
  currentPlanVersion?: number | undefined;
  onViewHistoricalPlan?: (artifactId: string, version: number) => void | undefined;
}

function ProposalCard({
  proposal,
  onSelect,
  onEdit,
  onRemove,
  isHighlighted = false,
  currentPlanVersion,
  onViewHistoricalPlan,
}: ProposalCardProps) {
  const effectivePriority = proposal.userPriority ?? proposal.suggestedPriority;
  const isSelected = proposal.selected;
  const priorityStyle = PRIORITY_STYLES[effectivePriority];

  // Check if we should show the historical plan link
  const showHistoricalPlanLink =
    proposal.planArtifactId &&
    proposal.planVersionAtCreation &&
    currentPlanVersion &&
    proposal.planVersionAtCreation !== currentPlanVersion;

  const handleViewHistoricalPlan = () => {
    if (proposal.planArtifactId && proposal.planVersionAtCreation && onViewHistoricalPlan) {
      onViewHistoricalPlan(proposal.planArtifactId, proposal.planVersionAtCreation);
    }
  };

  return (
    <Card
      data-testid={`proposal-card-${proposal.id}`}
      className={`group relative p-3 transition-all duration-150 cursor-pointer
        ${isHighlighted
          ? "border-2 border-yellow-500 bg-yellow-500/10 shadow-[0_0_0_3px_rgba(234,179,8,0.15)] animate-pulse"
          : isSelected
            ? "border-2 border-[var(--accent-primary)] bg-[rgba(255,107,53,0.05)] shadow-[0_0_0_3px_rgba(255,107,53,0.15)]"
            : "border border-[var(--border-subtle)] hover:shadow-[var(--shadow-sm)] hover:-translate-y-0.5"
        }`}
    >
      <div className="flex items-start gap-3">
        {/* Checkbox */}
        <div className="pt-0.5">
          <Checkbox
            checked={isSelected}
            onCheckedChange={() => onSelect(proposal.id)}
            aria-label={`Select ${proposal.title}`}
            className="data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
          />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {/* Title row */}
          <div className="flex items-start justify-between gap-2">
            <h3 className="text-sm font-medium text-[var(--text-primary)] leading-tight">
              {proposal.title}
            </h3>

            {/* Action buttons (visible on hover) */}
            <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={(e) => {
                        e.stopPropagation();
                        onEdit(proposal.id);
                      }}
                    >
                      <FileEdit className="w-3.5 h-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Edit</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={(e) => {
                        e.stopPropagation();
                        onRemove(proposal.id);
                      }}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Remove</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          {/* Description */}
          <p className="text-xs text-[var(--text-secondary)] mt-1 line-clamp-2">
            {proposal.description || "No description"}
          </p>

          {/* Badges row */}
          <div className="flex flex-wrap items-center gap-1.5 mt-2">
            <Badge className={`${priorityStyle.bg} ${priorityStyle.text} text-[10px] px-1.5 py-0`}>
              {effectivePriority.charAt(0).toUpperCase() + effectivePriority.slice(1)}
            </Badge>
            <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
              {proposal.category}
            </Badge>
            {proposal.userModified && (
              <Badge variant="outline" className="text-[10px] px-1.5 py-0 italic">
                Modified
              </Badge>
            )}
          </div>

          {/* Historical plan link */}
          {showHistoricalPlanLink && (
            <div className="mt-2">
              <button
                data-testid="view-historical-plan"
                onClick={(e) => {
                  e.stopPropagation();
                  handleViewHistoricalPlan();
                }}
                className="text-xs underline hover:no-underline transition-all flex items-center gap-1"
                style={{ color: "#ff6b35" }}
              >
                <Eye className="w-3 h-3" />
                View plan as of proposal creation (v{proposal.planVersionAtCreation})
              </button>
            </div>
          )}
        </div>
      </div>
    </Card>
  );
}

// ============================================================================
// Proactive Sync Notification
// ============================================================================

interface ProactiveSyncNotificationProps {
  notification: ProactiveSyncNotification;
  onDismiss: () => void;
  onReview: () => void;
  onUndo: () => void;
}

function ProactiveSyncNotificationBanner({
  notification,
  onDismiss,
  onReview,
  onUndo,
}: ProactiveSyncNotificationProps) {
  const affectedCount = notification.proposalIds.length;

  return (
    <Card
      data-testid="proactive-sync-notification"
      className="mb-4 border-2 border-[var(--accent-primary)] bg-[rgba(255,107,53,0.05)]"
    >
      <div className="p-3">
        <div className="flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-[var(--accent-primary)] flex-shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <p className="text-sm font-medium text-[var(--text-primary)] mb-1">
              Plan updated
            </p>
            <p className="text-sm text-[var(--text-secondary)]">
              {affectedCount} proposal{affectedCount !== 1 ? "s" : ""} may need revision.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={onReview}
                    className="text-[var(--accent-primary)] hover:bg-[rgba(255,107,53,0.1)]"
                  >
                    <Eye className="w-4 h-4 mr-1" />
                    Review
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Highlight affected proposals</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={onUndo}
                    className="text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]"
                  >
                    <Undo2 className="w-4 h-4 mr-1" />
                    Undo
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Revert proposals to previous state</TooltipContent>
              </Tooltip>
            </TooltipProvider>
            <Button
              variant="ghost"
              size="icon"
              onClick={onDismiss}
              className="h-7 w-7"
            >
              <span className="sr-only">Dismiss</span>
              ×
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}

// ============================================================================
// Proposals Panel Toolbar
// ============================================================================

interface ProposalsToolbarProps {
  selectedCount: number;
  totalCount: number;
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onSortByPriority: () => void;
  onClearAll: () => void;
}

function ProposalsToolbar({
  selectedCount,
  totalCount,
  onSelectAll,
  onDeselectAll,
  onSortByPriority,
  onClearAll,
}: ProposalsToolbarProps) {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-b border-[var(--border-subtle)]">
      <span className="text-xs text-[var(--text-secondary)]">
        {selectedCount} of {totalCount} selected
      </span>

      <div className="flex items-center gap-1">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onSelectAll}>
                <CheckSquare className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Select all</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onDeselectAll}>
                <Square className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Deselect all</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        <div className="w-px h-4 bg-[var(--border-subtle)] mx-1" />

        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onSortByPriority}>
                <ArrowUpDown className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Sort by priority</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7" onClick={onClearAll}>
                <Trash2 className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Clear all</TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>
    </div>
  );
}

// Removed - using ChatInput from @/components/chat/ChatInput instead

// ============================================================================
// Main Component
// ============================================================================

export function IdeationView({
  session,
  sessions,
  messages,
  proposals,
  onSendMessage,
  onNewSession,
  onSelectSession,
  onArchiveSession,
  onSelectProposal,
  onEditProposal,
  onRemoveProposal,
  onReorderProposals,
  onApply,
  isLoading = false,
}: IdeationViewProps) {
  const [leftPanelWidth, setLeftPanelWidth] = useState(50); // percentage
  const [isResizing, setIsResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Get plan artifact and settings from store
  const planArtifact = useIdeationStore((state) => state.planArtifact);
  const ideationSettings = useIdeationStore((state) => state.ideationSettings);
  const fetchPlanArtifact = useIdeationStore((state) => state.fetchPlanArtifact);
  const showSyncNotification = useIdeationStore((state) => state.showSyncNotification);
  const syncNotification = useIdeationStore((state) => state.syncNotification);
  const dismissSyncNotification = useIdeationStore((state) => state.dismissSyncNotification);

  // Fetch plan artifact when session changes and has planArtifactId
  useEffect(() => {
    if (session?.planArtifactId) {
      fetchPlanArtifact(session.planArtifactId);
    }
  }, [session?.planArtifactId, fetchPlanArtifact]);

  // Subscribe to proactive sync event
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      unlisten = await listen<{ artifact_id: string; proposal_ids: string[] }>(
        "plan:proposals_may_need_update",
        (event) => {
          // Store previous proposal states for undo
          const affectedProposals = proposals.filter((p) =>
            event.payload.proposal_ids.includes(p.id)
          );
          const previousStates: Record<string, unknown> = {};
          affectedProposals.forEach((p) => {
            previousStates[p.id] = { ...p };
          });

          // Show notification
          showSyncNotification({
            artifactId: event.payload.artifact_id,
            proposalIds: event.payload.proposal_ids,
            previousStates,
            timestamp: Date.now(),
          });
        }
      );
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [proposals, showSyncNotification]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (messagesEndRef.current && messagesEndRef.current.scrollIntoView) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages]);

  // Resize handling
  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  }, []);

  useEffect(() => {
    if (!isResizing) return;

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      const rect = containerRef.current.getBoundingClientRect();
      const newWidth = ((e.clientX - rect.left) / rect.width) * 100;
      // Enforce minimum width of 320px (approximately 30%)
      const minPercent = 30;
      const maxPercent = 70;
      setLeftPanelWidth(Math.max(minPercent, Math.min(maxPercent, newWidth)));
    };

    const handleMouseUp = () => {
      setIsResizing(false);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing]);

  const handleArchive = useCallback(() => {
    if (session) {
      onArchiveSession(session.id);
    }
  }, [session, onArchiveSession]);

  const handleApply = useCallback(
    (targetColumn: string) => {
      if (!session) return;

      const selectedProposals = proposals.filter((p) => p.selected);
      const options: ApplyProposalsInput = {
        sessionId: session.id,
        proposalIds: selectedProposals.map((p) => p.id),
        targetColumn,
        preserveDependencies: true,
      };
      onApply(options);
    },
    [session, proposals, onApply]
  );

  const handleSelectAll = useCallback(() => {
    proposals.forEach((p) => {
      if (!p.selected) {
        onSelectProposal(p.id);
      }
    });
  }, [proposals, onSelectProposal]);

  const handleDeselectAll = useCallback(() => {
    proposals.forEach((p) => {
      if (p.selected) {
        onSelectProposal(p.id);
      }
    });
  }, [proposals, onSelectProposal]);

  const handleSortByPriority = useCallback(() => {
    const sorted = [...proposals].sort((a, b) => b.priorityScore - a.priorityScore);
    onReorderProposals(sorted.map((p) => p.id));
  }, [proposals, onReorderProposals]);

  const handleClearAll = useCallback(() => {
    proposals.forEach((p) => {
      onRemoveProposal(p.id);
    });
  }, [proposals, onRemoveProposal]);

  // Proactive sync notification handlers
  const [highlightedProposalIds, setHighlightedProposalIds] = useState<Set<string>>(new Set());

  // Plan history dialog state
  const [planHistoryDialog, setPlanHistoryDialog] = useState<{
    isOpen: boolean;
    artifactId: string;
    version: number;
  } | null>(null);

  const handleViewHistoricalPlan = useCallback((artifactId: string, version: number) => {
    setPlanHistoryDialog({ isOpen: true, artifactId, version });
  }, []);

  const handleClosePlanHistoryDialog = useCallback(() => {
    setPlanHistoryDialog(null);
  }, []);

  const handleReviewSync = useCallback(() => {
    if (syncNotification) {
      setHighlightedProposalIds(new Set(syncNotification.proposalIds));
      // Auto-clear highlight after 5 seconds
      setTimeout(() => {
        setHighlightedProposalIds(new Set());
      }, 5000);
    }
  }, [syncNotification]);

  const handleUndoSync = useCallback(() => {
    if (!syncNotification) return;

    // Revert proposals to previous state
    // Note: This would require updating proposals via the parent component
    // For now, we'll dismiss the notification and log the undo action
    console.log("Undo sync - restoring proposals:", syncNotification.previousStates);

    // TODO: Implement actual proposal revert via parent component
    // This would require passing a callback from the parent to update proposal data

    dismissSyncNotification();
    setHighlightedProposalIds(new Set());
  }, [syncNotification, dismissSyncNotification]);

  const handleDismissSync = useCallback(() => {
    dismissSyncNotification();
    setHighlightedProposalIds(new Set());
  }, [dismissSyncNotification]);

  // Plan import handler
  const [importStatus, setImportStatus] = useState<{ type: "success" | "error"; message: string } | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleImportPlan = useCallback(() => {
    // Trigger the hidden file input
    fileInputRef.current?.click();
  }, []);

  const handleFileSelected = useCallback(async (event: React.ChangeEvent<HTMLInputElement>) => {
    if (!session) return;

    const file = event.target.files?.[0];
    if (!file) return;

    try {
      // Read file content
      const content = await file.text();

      // Extract title from filename
      const title = file.name.replace(/\.md$/, "").replace(/_/g, " ");

      // Call HTTP endpoint to create plan artifact
      const apiResponse = await fetch("http://localhost:3847/api/create_plan_artifact", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          session_id: session.id,
          title,
          content,
        }),
      });

      if (!apiResponse.ok) {
        throw new Error("Failed to import plan");
      }

      const data = await apiResponse.json();

      // Refresh plan artifact in store
      if (data.id) {
        await fetchPlanArtifact(data.id);

        // Show success notification
        setImportStatus({ type: "success", message: `Plan "${title}" imported successfully` });
        setTimeout(() => setImportStatus(null), 5000);
      }
    } catch (error) {
      console.error("Plan import error:", error);
      setImportStatus({
        type: "error",
        message: error instanceof Error ? error.message : "Failed to import plan",
      });
      setTimeout(() => setImportStatus(null), 5000);
    } finally {
      // Reset file input
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    }
  }, [session, fetchPlanArtifact]);

  const selectedCount = proposals.filter((p) => p.selected).length;
  const canApply = selectedCount > 0 && !isLoading;

  // Sort proposals by sortOrder
  const sortedProposals = useMemo(
    () => [...proposals].sort((a, b) => a.sortOrder - b.sortOrder),
    [proposals]
  );

  // Process messages into groups (for message grouping like ChatPanel)
  const groupedMessages = useMemo(() => {
    return messages.map((msg, index) => {
      const prevMsg = messages[index - 1];
      const nextMsg = messages[index + 1];
      const isFirstInGroup = !prevMsg || prevMsg.role !== msg.role;
      const isLastInGroup = !nextMsg || nextMsg.role !== msg.role;
      return { ...msg, isFirstInGroup, isLastInGroup };
    });
  }, [messages]);

  // Filter to active sessions only for sidebar
  const activeSessions = useMemo(
    () => sessions.filter((s) => s.status === "active"),
    [sessions]
  );

  return (
    <>
      <style>{animationStyles}</style>
      <div
        ref={containerRef}
        data-testid="ideation-view"
        className="flex h-full relative"
        style={{
          background:
            "radial-gradient(ellipse at top left, rgba(255,107,53,0.02) 0%, var(--bg-base) 40%)",
        }}
        role="main"
      >
        {/* Session Browser Sidebar - Always visible */}
        <SessionBrowser
          sessions={activeSessions}
          currentSessionId={session?.id ?? null}
          onSelectSession={onSelectSession}
          onNewSession={onNewSession}
        />

        {/* Main Content Area */}
        {!session ? (
          /* No session selected - show start panel */
          <StartSessionPanel onNewSession={onNewSession} />
        ) : (
          /* Active session - show conversation and proposals */
          <div className="flex flex-col flex-1 overflow-hidden">
            {/* Header with glass effect */}
            <header
              data-testid="ideation-header"
              className="flex items-center justify-between h-[52px] px-4 border-b border-[var(--border-subtle)]
                backdrop-blur-md bg-[rgba(26,26,26,0.85)]"
            >
              <h1 className="text-lg font-semibold text-[var(--text-primary)] tracking-tight truncate">
                {session.title ?? "New Session"}
              </h1>
              <div className="flex items-center gap-2">
                <Button variant="ghost" onClick={handleArchive} className="gap-2">
                  <Archive className="w-4 h-4" />
                  Archive
                </Button>
              </div>
            </header>

            {/* Main content - split layout */}
            <div data-testid="ideation-main-content" className="flex flex-1 overflow-hidden">
        {/* Conversation Panel (left) */}
        <div
          data-testid="conversation-panel"
          className="flex flex-col border-r border-[var(--border-subtle)] bg-[var(--bg-surface)]"
          style={{
            width: `${leftPanelWidth}%`,
            minWidth: "320px",
            boxShadow: "inset 0 0 80px rgba(0,0,0,0.1)",
          }}
        >
          {/* Panel Header */}
          <div className="flex items-center gap-2 px-4 py-2.5 h-10 backdrop-blur-sm bg-[rgba(26,26,26,0.7)] border-b border-[var(--border-subtle)]">
            <MessageSquare className="w-4 h-4 text-[var(--text-secondary)]" />
            <h2 className="text-sm font-semibold text-[var(--text-primary)]">Conversation</h2>
          </div>

          {/* Messages */}
          <div className="flex-1 overflow-y-auto p-4 scroll-smooth">
            {messages.length === 0 ? (
              <ConversationEmptyState />
            ) : (
              <>
                {groupedMessages.map((msg) => (
                  <MessageItem
                    key={msg.id}
                    role={msg.role}
                    content={msg.content}
                    createdAt={msg.createdAt}
                    toolCalls={msg.toolCalls}
                    isFirstInGroup={msg.isFirstInGroup}
                    isLastInGroup={msg.isLastInGroup}
                  />
                ))}
                {isLoading && <TypingIndicator />}
                <div ref={messagesEndRef} />
              </>
            )}
          </div>

          {/* Chat Input */}
          <div className="border-t border-[var(--border-subtle)] bg-[var(--bg-surface)] p-3">
            <ChatInput
              onSend={onSendMessage}
              isSending={isLoading}
              placeholder="Send a message..."
              showHelperText={true}
              autoFocus={false}
            />
          </div>
        </div>

        {/* Resize Handle */}
        <div
          data-testid="resize-handle"
          className={`w-1 cursor-ew-resize relative group ${
            isResizing ? "bg-[var(--accent-primary)]" : ""
          }`}
          onMouseDown={handleResizeStart}
        >
          <div
            className={`absolute top-0 bottom-0 left-1/2 -translate-x-1/2 w-px transition-all duration-150
              ${isResizing ? "bg-[var(--accent-primary)] shadow-[0_0_8px_rgba(255,107,53,0.3)]" : "bg-[var(--border-subtle)] group-hover:bg-[var(--accent-primary)] group-hover:shadow-[0_0_8px_rgba(255,107,53,0.3)]"}`}
          />
        </div>

        {/* Proposals Panel (right) */}
        <div
          data-testid="proposals-panel"
          className="flex flex-col flex-1 bg-[var(--bg-surface)]"
          style={{ minWidth: "320px" }}
        >
          {/* Panel Header */}
          <div className="flex items-center justify-between px-4 py-2.5 h-10 border-b border-[var(--border-subtle)]">
            <div className="flex items-center gap-2">
              <ListTodo className="w-4 h-4 text-[var(--text-secondary)]" />
              <h2 className="text-sm font-semibold text-[var(--text-primary)]">Task Proposals</h2>
            </div>
            <Badge variant="secondary">{proposals.length}</Badge>
          </div>

          {/* Toolbar */}
          {proposals.length > 0 && (
            <ProposalsToolbar
              selectedCount={selectedCount}
              totalCount={proposals.length}
              onSelectAll={handleSelectAll}
              onDeselectAll={handleDeselectAll}
              onSortByPriority={handleSortByPriority}
              onClearAll={handleClearAll}
            />
          )}

          {/* Proposals List with Plan Display */}
          <div className="flex-1 overflow-y-auto p-4">
            {/* Import Status Notification */}
            {importStatus && (
              <Card
                data-testid="import-status-notification"
                className={`mb-4 border-2 ${
                  importStatus.type === "success"
                    ? "border-green-500 bg-green-500/10"
                    : "border-red-500 bg-red-500/10"
                }`}
              >
                <div className="p-3 flex items-center justify-between">
                  <p className="text-sm font-medium text-[var(--text-primary)]">
                    {importStatus.message}
                  </p>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => setImportStatus(null)}
                    className="h-7 w-7"
                  >
                    ×
                  </Button>
                </div>
              </Card>
            )}

            {/* Proactive Sync Notification */}
            {syncNotification && (
              <ProactiveSyncNotificationBanner
                notification={syncNotification}
                onDismiss={handleDismissSync}
                onReview={handleReviewSync}
                onUndo={handleUndoSync}
              />
            )}

            {/* Import Plan Button - shown when no plan exists */}
            {!planArtifact && proposals.length > 0 && (
              <div className="mb-4">
                <Button
                  variant="outline"
                  onClick={handleImportPlan}
                  className="w-full gap-2"
                  data-testid="import-plan-button"
                >
                  <Upload className="w-4 h-4" />
                  Import Implementation Plan
                </Button>
              </div>
            )}

            {/* Plan Display - shown above proposals when plan exists */}
            {planArtifact && (
              <div className="mb-4">
                <PlanDisplay
                  plan={planArtifact}
                  showApprove={ideationSettings?.requirePlanApproval ?? false}
                  linkedProposalsCount={
                    proposals.filter(
                      (p) => p.planArtifactId === planArtifact.id
                    ).length
                  }
                  onEdit={() => {
                    // TODO: Implement plan editor modal/panel
                    console.log("Edit plan:", planArtifact.id);
                  }}
                />
              </div>
            )}

            {/* Waiting for plan message when no plan in Required mode */}
            {!planArtifact &&
              ideationSettings?.planMode === "required" &&
              proposals.length === 0 && (
                <div className="flex flex-col items-center justify-center h-full p-12 text-center">
                  <div className="p-6 rounded-lg border-2 border-dashed border-[var(--border-subtle)]">
                    <Loader2 className="w-12 h-12 mx-auto mb-4 text-[var(--text-muted)] animate-spin" />
                    <p className="font-medium text-[var(--text-secondary)]">
                      Waiting for implementation plan...
                    </p>
                    <p className="text-sm text-[var(--text-muted)] mt-1">
                      The orchestrator will create a plan before proposing tasks
                    </p>
                  </div>
                </div>
              )}

            {/* Proposals Empty State (when not waiting for plan) */}
            {proposals.length === 0 &&
              !(
                !planArtifact &&
                ideationSettings?.planMode === "required"
              ) && <ProposalsEmptyState />}

            {/* Proposals List */}
            {proposals.length > 0 && (
              <div className="space-y-2">
                {sortedProposals.map((proposal) => (
                  <ProposalCard
                    key={proposal.id}
                    proposal={proposal}
                    onSelect={onSelectProposal}
                    onEdit={onEditProposal}
                    onRemove={onRemoveProposal}
                    isHighlighted={highlightedProposalIds.has(proposal.id)}
                    currentPlanVersion={planArtifact?.metadata.version ?? undefined}
                    onViewHistoricalPlan={handleViewHistoricalPlan}
                  />
                ))}
              </div>
            )}
          </div>

          {/* Apply Section */}
          <div
            data-testid="apply-section"
            className="flex items-center justify-between px-4 py-3 h-14 border-t border-[var(--border-subtle)] bg-[var(--bg-surface)]"
          >
            <span className="text-sm text-[var(--text-secondary)]">
              {selectedCount} selected
            </span>

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button disabled={!canApply}>
                  Apply to
                  <ChevronDown className="w-4 h-4 ml-1" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => handleApply("draft")}>
                  <FileEdit className="w-4 h-4 mr-2" />
                  Draft
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => handleApply("backlog")}>
                  <Inbox className="w-4 h-4 mr-2" />
                  Backlog
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => handleApply("todo")}>
                  <ListTodo className="w-4 h-4 mr-2" />
                  Todo
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
            </div>
          </div>
        )}

        {/* Plan History Dialog */}
        {planHistoryDialog && (
          <PlanHistoryDialog
            isOpen={planHistoryDialog.isOpen}
            onClose={handleClosePlanHistoryDialog}
            artifactId={planHistoryDialog.artifactId}
            version={planHistoryDialog.version}
          />
        )}

        {/* Hidden file input for plan import */}
        <input
          ref={fileInputRef}
          type="file"
          accept=".md"
          onChange={handleFileSelected}
          className="hidden"
          data-testid="plan-import-file-input"
        />
      </div>
    </>
  );
}
