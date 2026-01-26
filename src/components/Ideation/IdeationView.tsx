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
  Send,
  Paperclip,
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
} from "lucide-react";
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
import { useIdeationStore } from "@/stores/ideationStore";

// ============================================================================
// Types
// ============================================================================

interface IdeationViewProps {
  session: IdeationSession | null;
  messages: ChatMessageType[];
  proposals: TaskProposal[];
  onSendMessage: (content: string) => void;
  onNewSession: () => void;
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

function TypingIndicator() {
  return (
    <div
      data-testid="typing-indicator"
      className="flex items-start mb-3"
    >
      <div
        className="px-4 py-3 rounded-xl bg-[var(--bg-elevated)] border border-[var(--border-subtle)]"
        style={{ borderRadius: "12px 12px 12px 4px" }}
      >
        <div className="flex items-center gap-1">
          <span
            className="w-1.5 h-1.5 rounded-full bg-[var(--text-muted)] animate-bounce"
            style={{ animationDelay: "0ms", animationDuration: "1.4s" }}
          />
          <span
            className="w-1.5 h-1.5 rounded-full bg-[var(--text-muted)] animate-bounce"
            style={{ animationDelay: "100ms", animationDuration: "1.4s" }}
          />
          <span
            className="w-1.5 h-1.5 rounded-full bg-[var(--text-muted)] animate-bounce"
            style={{ animationDelay: "200ms", animationDuration: "1.4s" }}
          />
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Chat Message Bubble
// ============================================================================

interface MessageBubbleProps {
  message: ChatMessageType;
}

function MessageBubble({ message }: MessageBubbleProps) {
  const isUser = message.role === "user";

  const timestamp = useMemo(() => {
    const date = new Date(message.createdAt);
    return date.toLocaleTimeString([], {
      hour: "numeric",
      minute: "2-digit",
    });
  }, [message.createdAt]);

  return (
    <div
      data-testid={`chat-message-${message.id}`}
      className={`flex flex-col mb-3 ${isUser ? "items-end" : "items-start"}`}
    >
      <div
        className={`max-w-[85%] px-4 py-3 ${
          isUser
            ? "bg-[var(--accent-primary)] text-white"
            : "bg-[var(--bg-elevated)] border border-[var(--border-subtle)] text-[var(--text-primary)]"
        }`}
        style={{
          borderRadius: isUser ? "12px 12px 4px 12px" : "12px 12px 12px 4px",
          boxShadow: isUser ? "var(--shadow-xs)" : "none",
        }}
      >
        <div className="text-sm">
          <ReactMarkdown components={markdownComponents}>
            {message.content}
          </ReactMarkdown>
        </div>
      </div>
      <time
        className={`text-[11px] mt-1 px-1 text-[var(--text-muted)] ${
          isUser ? "text-right" : "text-left"
        }`}
      >
        {timestamp}
      </time>
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
// No Session State
// ============================================================================

function NoSessionState({ onNewSession }: { onNewSession: () => void }) {
  return (
    <div
      data-testid="ideation-view"
      className="flex flex-col items-center justify-center h-full p-8"
      style={{
        backgroundColor: "var(--bg-base)",
        background:
          "radial-gradient(ellipse at top left, rgba(255,107,53,0.02) 0%, var(--bg-base) 40%)",
      }}
      role="main"
    >
      <div className="p-8 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-center shadow-[var(--shadow-sm)]">
        <Lightbulb className="w-12 h-12 mx-auto mb-4 text-[var(--accent-primary)]" />
        <h2 className="text-xl font-semibold mb-2 text-[var(--text-primary)] tracking-tight">
          Start a new ideation session
        </h2>
        <p className="text-sm mb-6 text-[var(--text-secondary)]">
          Brainstorm ideas and create task proposals
        </p>
        <Button onClick={onNewSession} className="px-6">
          <Plus className="w-4 h-4 mr-2" />
          Start Session
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
}

function ProposalCard({ proposal, onSelect, onEdit, onRemove }: ProposalCardProps) {
  const effectivePriority = proposal.userPriority ?? proposal.suggestedPriority;
  const isSelected = proposal.selected;
  const priorityStyle = PRIORITY_STYLES[effectivePriority];

  return (
    <Card
      data-testid={`proposal-card-${proposal.id}`}
      className={`group relative p-3 transition-all duration-150 cursor-pointer
        ${isSelected
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

// ============================================================================
// Chat Input (Premium)
// ============================================================================

interface PremiumChatInputProps {
  onSend: (message: string) => void;
  isSending: boolean;
}

function PremiumChatInput({ onSend, isSending }: PremiumChatInputProps) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const canSend = value.trim().length > 0 && !isSending;

  const handleSend = useCallback(() => {
    if (canSend) {
      onSend(value.trim());
      setValue("");
    }
  }, [canSend, value, onSend]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
      const newHeight = Math.min(120, Math.max(44, textareaRef.current.scrollHeight));
      textareaRef.current.style.height = `${newHeight}px`;
    }
  }, [value]);

  return (
    <div className="border-t border-[var(--border-subtle)] bg-[var(--bg-surface)] p-3">
      <div className="flex items-end gap-2">
        <Button variant="ghost" size="icon" disabled className="shrink-0 h-11 w-11 opacity-50">
          <Paperclip className="w-5 h-5" />
        </Button>

        <textarea
          ref={textareaRef}
          data-testid="chat-input-textarea"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={isSending}
          placeholder="Send a message..."
          rows={1}
          className="flex-1 px-4 py-3 text-sm resize-none rounded-lg outline-none transition-shadow
            bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--border-default)]
            focus:border-[var(--accent-primary)] focus:shadow-[var(--shadow-glow)]
            placeholder:text-[var(--text-muted)]"
          style={{ minHeight: "44px", maxHeight: "120px" }}
        />

        <Button
          data-testid="chat-input-send"
          size="icon"
          disabled={!canSend}
          onClick={handleSend}
          className="shrink-0 h-11 w-11"
        >
          {isSending ? (
            <Loader2 className="w-5 h-5 animate-spin" />
          ) : (
            <Send className="w-5 h-5" />
          )}
        </Button>
      </div>
      <p className="text-[11px] text-[var(--text-muted)] mt-1.5 ml-14">
        Enter to send, Shift+Enter for new line
      </p>
    </div>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function IdeationView({
  session,
  messages,
  proposals,
  onSendMessage,
  onNewSession,
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

  // Fetch plan artifact when session changes and has planArtifactId
  useEffect(() => {
    if (session?.planArtifactId) {
      fetchPlanArtifact(session.planArtifactId);
    }
  }, [session?.planArtifactId, fetchPlanArtifact]);

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

  const selectedCount = proposals.filter((p) => p.selected).length;
  const canApply = selectedCount > 0 && !isLoading;

  // Sort proposals by sortOrder
  const sortedProposals = useMemo(
    () => [...proposals].sort((a, b) => a.sortOrder - b.sortOrder),
    [proposals]
  );

  // No session state
  if (!session) {
    return <NoSessionState onNewSession={onNewSession} />;
  }

  return (
    <div
      ref={containerRef}
      data-testid="ideation-view"
      className="flex flex-col h-full relative"
      style={{
        background:
          "radial-gradient(ellipse at top left, rgba(255,107,53,0.02) 0%, var(--bg-base) 40%)",
      }}
      role="main"
    >
      {/* Loading overlay */}
      {isLoading && (
        <div
          data-testid="ideation-loading"
          className="absolute inset-0 flex items-center justify-center z-50 bg-black/30 backdrop-blur-sm"
        >
          <div className="px-6 py-4 rounded-lg bg-[var(--bg-elevated)] text-[var(--text-primary)] shadow-[var(--shadow-md)] flex items-center gap-3">
            <Loader2 className="w-5 h-5 animate-spin text-[var(--accent-primary)]" />
            <span className="text-sm font-medium">Processing...</span>
          </div>
        </div>
      )}

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
          <Button variant="ghost" onClick={onNewSession} className="gap-2">
            <Plus className="w-4 h-4" />
            New Session
          </Button>
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
                {messages.map((message) => (
                  <MessageBubble key={message.id} message={message} />
                ))}
                {isLoading && <TypingIndicator />}
                <div ref={messagesEndRef} />
              </>
            )}
          </div>

          {/* Chat Input */}
          <PremiumChatInput onSend={onSendMessage} isSending={isLoading} />
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
  );
}
