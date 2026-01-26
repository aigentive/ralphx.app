/**
 * IdeationView - Premium Studio-Grade Ideation Interface
 *
 * Design: "Refined Studio" - Luxurious dark interface with sophisticated
 * depth layers, editorial typography, and warm orange jewel accents.
 *
 * Features:
 * - Persistent session browser sidebar with elegant cards
 * - Two-panel resizable layout with smooth drag handle
 * - Premium message bubbles with glass effects
 * - Sophisticated proposal cards with hover states
 * - Atmospheric backgrounds with subtle grain
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
  Sparkles,
  ArrowRight,
  Layers,
  Zap,
} from "lucide-react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  IdeationSession,
  TaskProposal,
  ChatMessage as ChatMessageType,
  ApplyProposalsInput,
} from "@/types/ideation";
import { Button } from "@/components/ui/button";
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
// Design Tokens & Animation Styles
// ============================================================================

const PRIORITY_CONFIG: Record<Priority, { gradient: string; glow: string; label: string }> = {
  critical: {
    gradient: "from-red-500/20 to-red-600/10",
    glow: "shadow-[0_0_12px_rgba(239,68,68,0.1)]",
    label: "Critical"
  },
  high: {
    gradient: "from-[#ff6b35]/20 to-[#ff6b35]/10",
    glow: "shadow-[0_0_12px_rgba(255,107,53,0.1)]",
    label: "High"
  },
  medium: {
    gradient: "from-amber-500/15 to-amber-600/5",
    glow: "",
    label: "Medium"
  },
  low: {
    gradient: "from-slate-500/10 to-slate-600/5",
    glow: "",
    label: "Low"
  },
};

const animationStyles = `
@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

@keyframes subtlePulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}

@keyframes shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

@keyframes fadeSlideIn {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes glowPulse {
  0%, 100% {
    box-shadow: 0 0 12px rgba(255,107,53,0.08),
                0 0 24px rgba(255,107,53,0.04),
                inset 0 1px 0 rgba(255,255,255,0.05);
  }
  50% {
    box-shadow: 0 0 18px rgba(255,107,53,0.15),
                0 0 36px rgba(255,107,53,0.08),
                inset 0 1px 0 rgba(255,255,255,0.08);
  }
}

.typing-dot {
  animation: typingBounce 1.4s ease-in-out infinite;
}
.typing-dot:nth-child(2) { animation-delay: 0.15s; }
.typing-dot:nth-child(3) { animation-delay: 0.3s; }

.session-card-enter {
  animation: fadeSlideIn 0.3s ease-out forwards;
}

.active-session-glow {
  animation: glowPulse 3s ease-in-out infinite;
}

.shimmer-loading {
  background: linear-gradient(
    90deg,
    rgba(255,255,255,0) 0%,
    rgba(255,255,255,0.05) 50%,
    rgba(255,255,255,0) 100%
  );
  background-size: 200% 100%;
  animation: shimmer 2s infinite;
}
`;

// ============================================================================
// Markdown Components
// ============================================================================

const markdownComponents = {
  a: ({ href, children, ...props }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="text-[#ff6b35] hover:text-[#ff8050] underline decoration-[#ff6b35]/30 hover:decoration-[#ff6b35]/60 transition-colors"
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
          className={cn(
            "block p-4 rounded-lg text-[13px] leading-relaxed overflow-x-auto",
            "bg-black/40 border border-white/5",
            "font-mono",
            className
          )}
          {...props}
        >
          {children}
        </code>
      );
    }
    return (
      <code
        className="px-1.5 py-0.5 rounded text-[13px] bg-white/5 border border-white/5 font-mono text-[#ffa94d]"
        {...props}
      >
        {children}
      </code>
    );
  },
  pre: ({ children, ...props }: React.HTMLAttributes<HTMLPreElement>) => (
    <pre className="my-3 rounded-lg overflow-hidden" {...props}>
      {children}
    </pre>
  ),
  p: ({ children, ...props }: React.HTMLAttributes<HTMLParagraphElement>) => (
    <p className="mb-2.5 last:mb-0 leading-relaxed" {...props}>
      {children}
    </p>
  ),
  ul: ({ children, ...props }: React.HTMLAttributes<HTMLUListElement>) => (
    <ul className="list-none space-y-1.5 mb-3" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }: React.HTMLAttributes<HTMLOListElement>) => (
    <ol className="list-decimal list-inside mb-3 space-y-1" {...props}>
      {children}
    </ol>
  ),
  li: ({ children, ...props }: React.LiHTMLAttributes<HTMLLIElement>) => (
    <li className="flex items-start gap-2" {...props}>
      <span className="w-1.5 h-1.5 rounded-full bg-[#ff6b35]/60 mt-2 flex-shrink-0" />
      <span>{children}</span>
    </li>
  ),
};

// ============================================================================
// Typing Indicator (Premium)
// ============================================================================

function TypingIndicator() {
  return (
    <div data-testid="typing-indicator" className="flex items-start gap-3 mb-4">
      <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/5 flex items-center justify-center border border-[#ff6b35]/20">
        <Bot className="w-4 h-4 text-[#ff6b35]" />
      </div>
      <div
        className="px-4 py-3 rounded-2xl rounded-tl-md bg-gradient-to-br from-white/[0.03] to-white/[0.01] border border-white/[0.06] backdrop-blur-sm"
      >
        <div className="flex items-center gap-1.5">
          {[0, 1, 2].map((i) => (
            <div
              key={i}
              className="typing-dot w-2 h-2 rounded-full bg-[#ff6b35]"
              style={{ animationDelay: `${i * 0.15}s` }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Message Bubble (Premium)
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

    return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  }, [createdAt]);

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
        "flex session-card-enter",
        isUser ? "justify-end" : "justify-start",
        isLastInGroup ? "mb-4" : "mb-1.5"
      )}
    >
      {/* Agent avatar */}
      {!isUser && isFirstInGroup && (
        <div className="w-8 h-8 rounded-full bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/5 flex items-center justify-center border border-[#ff6b35]/20 mr-3 flex-shrink-0">
          <Bot className="w-4 h-4 text-[#ff6b35]" />
        </div>
      )}
      {!isUser && !isFirstInGroup && <div className="w-8 mr-3 flex-shrink-0" />}

      <div className="flex flex-col max-w-[80%]">
        {/* Tool calls */}
        {!isUser && parsedToolCalls.length > 0 && (
          <div className="space-y-2 mb-2">
            {parsedToolCalls.map((tc) => (
              <ToolCallIndicator key={tc.id} toolCall={tc} />
            ))}
          </div>
        )}

        {/* Message bubble */}
        <div
          className={cn(
            "px-4 py-3 text-[14px] leading-relaxed",
            isUser
              ? "rounded-2xl rounded-tr-md bg-gradient-to-br from-[#ff6b35] to-[#e55a2b] text-white shadow-lg shadow-[#ff6b35]/20"
              : "rounded-2xl rounded-tl-md bg-gradient-to-br from-white/[0.04] to-white/[0.01] border border-white/[0.06] backdrop-blur-sm"
          )}
        >
          {isUser ? (
            <p className="whitespace-pre-wrap break-words">{content}</p>
          ) : (
            <div className="prose prose-sm prose-invert max-w-none text-[var(--text-primary)]">
              <ReactMarkdown components={markdownComponents}>{content}</ReactMarkdown>
            </div>
          )}
        </div>

        {isLastInGroup && (
          <span className={cn("text-[11px] mt-1.5 px-1 text-[var(--text-muted)]", isUser ? "text-right" : "text-left")}>
            {timestamp}
          </span>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Empty States (Premium)
// ============================================================================

function ConversationEmptyState() {
  return (
    <div className="flex flex-col items-center justify-center h-full p-8">
      <div className="relative">
        {/* Glow effect */}
        <div className="absolute inset-0 bg-[#ff6b35]/10 rounded-3xl blur-3xl" />

        <div className="relative p-10 rounded-2xl bg-gradient-to-br from-white/[0.03] to-transparent border border-white/[0.06] backdrop-blur-sm text-center">
          <div className="w-16 h-16 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/5 flex items-center justify-center border border-[#ff6b35]/20">
            <MessageSquareText className="w-8 h-8 text-[#ff6b35]" />
          </div>
          <h3 className="text-lg font-semibold text-[var(--text-primary)] mb-2 tracking-tight">
            Start the conversation
          </h3>
          <p className="text-sm text-[var(--text-secondary)] max-w-[240px] leading-relaxed">
            Describe your ideas and I'll help create actionable task proposals
          </p>
        </div>
      </div>
    </div>
  );
}

function ProposalsEmptyState() {
  return (
    <div data-testid="proposals-empty-state" className="flex flex-col items-center justify-center h-full p-8">
      <div className="relative">
        <div className="absolute inset-0 bg-amber-500/5 rounded-3xl blur-3xl" />

        <div className="relative p-10 rounded-2xl bg-gradient-to-br from-white/[0.03] to-transparent border border-white/[0.06] backdrop-blur-sm text-center">
          <div className="w-16 h-16 mx-auto mb-6 rounded-2xl bg-gradient-to-br from-amber-500/15 to-amber-500/5 flex items-center justify-center border border-amber-500/20">
            <Lightbulb className="w-8 h-8 text-amber-400" />
          </div>
          <h3 className="text-lg font-semibold text-[var(--text-primary)] mb-2 tracking-tight">
            No proposals yet
          </h3>
          <p className="text-sm text-[var(--text-secondary)] max-w-[240px] leading-relaxed">
            Chat with the orchestrator to generate task proposals
          </p>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Session Browser (Premium Sidebar)
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

function SessionBrowser({ sessions, currentSessionId, onSelectSession, onNewSession }: SessionBrowserProps) {
  const sortedSessions = useMemo(
    () => [...sessions].sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()),
    [sessions]
  );

  return (
    <div
      data-testid="session-browser"
      className="flex flex-col h-full bg-[#0a0a0a] border-r border-white/[0.06]"
      style={{ width: "300px", minWidth: "300px", flexShrink: 0 }}
    >
      {/* Header */}
      <div className="px-5 py-4 border-b border-white/[0.06]">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/5 flex items-center justify-center border border-[#ff6b35]/20">
              <Layers className="w-4 h-4 text-[#ff6b35]" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-[var(--text-primary)] tracking-tight">Sessions</h2>
              <p className="text-[11px] text-[var(--text-muted)]">{sessions.length} total</p>
            </div>
          </div>
        </div>

        {/* New Session Button */}
        <Button
          onClick={onNewSession}
          className="w-full h-10 bg-gradient-to-r from-[#ff6b35] to-[#e55a2b] hover:from-[#ff7a4a] hover:to-[#ff6b35] text-white font-medium shadow-lg shadow-[#ff6b35]/20 border-0 transition-all duration-200"
        >
          <Plus className="w-4 h-4 mr-2" />
          New Session
        </Button>
      </div>

      {/* Session List */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {sortedSessions.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full p-6 text-center">
            <div className="w-12 h-12 rounded-xl bg-white/[0.03] flex items-center justify-center mb-3 border border-white/[0.06]">
              <Sparkles className="w-5 h-5 text-[var(--text-muted)]" />
            </div>
            <p className="text-sm text-[var(--text-muted)]">No sessions yet</p>
            <p className="text-xs text-[var(--text-muted)] mt-1">Start your first brainstorm</p>
          </div>
        ) : (
          sortedSessions.map((session, index) => {
            const isSelected = session.id === currentSessionId;
            return (
              <button
                key={session.id}
                data-testid={`session-item-${session.id}`}
                onClick={() => onSelectSession(session.id)}
                className={cn(
                  "session-card-enter w-full p-4 rounded-xl text-left transition-all duration-200",
                  "border border-transparent",
                  "hover:bg-white/[0.03] hover:border-white/[0.06]",
                  isSelected && "bg-gradient-to-br from-[#ff6b35]/10 to-[#ff6b35]/5 border-[#ff6b35]/30 active-session-glow"
                )}
                style={{ animationDelay: `${index * 50}ms` }}
              >
                <div className="flex items-start gap-3">
                  {/* Session indicator */}
                  <div className={cn(
                    "w-10 h-10 rounded-lg flex items-center justify-center flex-shrink-0 transition-colors",
                    isSelected
                      ? "bg-gradient-to-br from-[#ff6b35]/30 to-[#ff6b35]/10 border border-[#ff6b35]/30"
                      : "bg-white/[0.03] border border-white/[0.06]"
                  )}>
                    <MessageSquare className={cn("w-4 h-4", isSelected ? "text-[#ff6b35]" : "text-[var(--text-muted)]")} />
                  </div>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className={cn(
                        "text-sm font-medium truncate",
                        isSelected ? "text-[var(--text-primary)]" : "text-[var(--text-secondary)]"
                      )}>
                        {session.title || "Untitled Session"}
                      </span>
                      {isSelected && (
                        <span className="w-1.5 h-1.5 rounded-full bg-[#ff6b35] flex-shrink-0" />
                      )}
                    </div>
                    <div className="flex items-center gap-1.5 text-[11px] text-[var(--text-muted)]">
                      <Clock className="w-3 h-3" />
                      <span>{formatRelativeTime(session.updatedAt)}</span>
                    </div>
                  </div>

                  {/* Arrow indicator */}
                  <ArrowRight className={cn(
                    "w-4 h-4 flex-shrink-0 transition-all duration-200",
                    isSelected ? "text-[#ff6b35] translate-x-0 opacity-100" : "text-[var(--text-muted)] -translate-x-1 opacity-0 group-hover:translate-x-0 group-hover:opacity-100"
                  )} />
                </div>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Start Session Panel (Premium)
// ============================================================================

function StartSessionPanel({ onNewSession }: { onNewSession: () => void }) {
  return (
    <div className="flex-1 flex flex-col items-center justify-center p-8 relative overflow-hidden">
      {/* Background effects */}
      <div className="absolute inset-0 bg-gradient-to-br from-[#ff6b35]/[0.02] via-transparent to-purple-500/[0.02]" />
      <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-[#ff6b35]/5 rounded-full blur-[120px]" />
      <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-amber-500/5 rounded-full blur-[120px]" />

      <div className="relative z-10 text-center max-w-lg">
        {/* Icon */}
        <div className="relative mx-auto mb-8">
          <div className="absolute inset-0 bg-[#ff6b35]/20 rounded-3xl blur-2xl" />
          <div className="relative w-24 h-24 rounded-2xl bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/5 flex items-center justify-center border border-[#ff6b35]/20 mx-auto">
            <Lightbulb className="w-12 h-12 text-[#ff6b35]" />
          </div>
        </div>

        {/* Content */}
        <h1 className="text-3xl font-bold text-[var(--text-primary)] mb-4 tracking-tight">
          Ideation Studio
        </h1>
        <p className="text-lg text-[var(--text-secondary)] mb-8 leading-relaxed">
          Select a session from the sidebar to continue your work, or start a fresh brainstorming session.
        </p>

        {/* Action button */}
        <Button
          onClick={onNewSession}
          size="lg"
          className="h-12 px-8 bg-gradient-to-r from-[#ff6b35] to-[#e55a2b] hover:from-[#ff7a4a] hover:to-[#ff6b35] text-white font-semibold shadow-xl shadow-[#ff6b35]/25 border-0 transition-all duration-200"
        >
          <Zap className="w-5 h-5 mr-2" />
          Start New Session
        </Button>

        {/* Hint */}
        <p className="text-sm text-[var(--text-muted)] mt-6">
          Press <kbd className="px-2 py-0.5 rounded bg-white/[0.05] border border-white/[0.1] text-[11px] font-mono">⌘ N</kbd> to quickly start a new session
        </p>
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
  const config = PRIORITY_CONFIG[effectivePriority];

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
    <div
      data-testid={`proposal-card-${proposal.id}`}
      className={cn(
        "group relative p-4 rounded-xl transition-all duration-200 cursor-pointer session-card-enter",
        "bg-gradient-to-br",
        config.gradient,
        "border",
        isHighlighted
          ? "border-yellow-500/50 shadow-[0_0_30px_rgba(234,179,8,0.2)]"
          : isSelected
            ? "border-[#ff6b35]/40 shadow-[0_0_30px_rgba(255,107,53,0.15)]"
            : "border-white/[0.06] hover:border-white/[0.1] hover:shadow-lg hover:shadow-black/20",
        config.glow
      )}
      onClick={() => onSelect(proposal.id)}
    >
      {/* Selection indicator bar */}
      <div className={cn(
        "absolute left-0 top-3 bottom-3 w-1 rounded-full transition-all duration-200",
        isSelected ? "bg-[#ff6b35]" : "bg-transparent"
      )} />

      <div className="flex items-start gap-3 pl-2">
        {/* Checkbox */}
        <div className="pt-0.5">
          <Checkbox
            checked={isSelected}
            onCheckedChange={() => onSelect(proposal.id)}
            aria-label={`Select ${proposal.title}`}
            className="data-[state=checked]:bg-[#ff6b35] data-[state=checked]:border-[#ff6b35] border-white/20"
          />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-start justify-between gap-2">
            <h3 className="text-sm font-medium text-[var(--text-primary)] leading-snug">
              {proposal.title}
            </h3>

            {/* Actions */}
            <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 hover:bg-white/[0.06]"
                      onClick={(e) => { e.stopPropagation(); onEdit(proposal.id); }}
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
                      className="h-7 w-7 hover:bg-red-500/10 hover:text-red-400"
                      onClick={(e) => { e.stopPropagation(); onRemove(proposal.id); }}
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Remove</TooltipContent>
                </Tooltip>
              </TooltipProvider>
            </div>
          </div>

          <p className="text-xs text-[var(--text-secondary)] mt-1.5 line-clamp-2 leading-relaxed">
            {proposal.description || "No description"}
          </p>

          {/* Tags */}
          <div className="flex flex-wrap items-center gap-2 mt-3">
            <span className={cn(
              "px-2 py-0.5 rounded-md text-[10px] font-medium uppercase tracking-wider",
              effectivePriority === "critical" && "bg-red-500/20 text-red-400",
              effectivePriority === "high" && "bg-[#ff6b35]/20 text-[#ff6b35]",
              effectivePriority === "medium" && "bg-amber-500/20 text-amber-400",
              effectivePriority === "low" && "bg-slate-500/20 text-slate-400"
            )}>
              {config.label}
            </span>
            <span className="px-2 py-0.5 rounded-md text-[10px] font-medium bg-white/[0.05] text-[var(--text-muted)] border border-white/[0.06]">
              {proposal.category}
            </span>
            {proposal.userModified && (
              <span className="px-2 py-0.5 rounded-md text-[10px] font-medium bg-purple-500/20 text-purple-400 italic">
                Modified
              </span>
            )}
          </div>

          {showHistoricalPlanLink && (
            <button
              data-testid="view-historical-plan"
              onClick={(e) => { e.stopPropagation(); handleViewHistoricalPlan(); }}
              className="mt-3 text-xs text-[#ff6b35] hover:text-[#ff8050] flex items-center gap-1.5 transition-colors"
            >
              <Eye className="w-3 h-3" />
              View plan as of proposal creation (v{proposal.planVersionAtCreation})
            </button>
          )}
        </div>
      </div>
    </div>
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

function ProactiveSyncNotificationBanner({ notification, onDismiss, onReview, onUndo }: ProactiveSyncNotificationProps) {
  const affectedCount = notification.proposalIds.length;

  return (
    <div
      data-testid="proactive-sync-notification"
      className="mb-4 p-4 rounded-xl bg-gradient-to-br from-[#ff6b35]/10 to-[#ff6b35]/5 border border-[#ff6b35]/30"
    >
      <div className="flex items-start gap-3">
        <div className="w-10 h-10 rounded-lg bg-[#ff6b35]/20 flex items-center justify-center flex-shrink-0">
          <AlertCircle className="w-5 h-5 text-[#ff6b35]" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-[var(--text-primary)] mb-1">Plan updated</p>
          <p className="text-sm text-[var(--text-secondary)]">
            {affectedCount} proposal{affectedCount !== 1 ? "s" : ""} may need revision.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={onReview} className="text-[#ff6b35] hover:bg-[#ff6b35]/10">
            <Eye className="w-4 h-4 mr-1" /> Review
          </Button>
          <Button variant="ghost" size="sm" onClick={onUndo} className="hover:bg-white/[0.06]">
            <Undo2 className="w-4 h-4 mr-1" /> Undo
          </Button>
          <Button variant="ghost" size="icon" onClick={onDismiss} className="h-7 w-7 hover:bg-white/[0.06]">
            ×
          </Button>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Proposals Toolbar
// ============================================================================

interface ProposalsToolbarProps {
  selectedCount: number;
  totalCount: number;
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onSortByPriority: () => void;
  onClearAll: () => void;
}

function ProposalsToolbar({ selectedCount, totalCount, onSelectAll, onDeselectAll, onSortByPriority, onClearAll }: ProposalsToolbarProps) {
  return (
    <div className="flex items-center justify-between px-5 py-3 border-b border-white/[0.06] bg-black/20">
      <span className="text-xs text-[var(--text-muted)]">
        <span className="text-[var(--text-primary)] font-medium">{selectedCount}</span> of {totalCount} selected
      </span>

      <div className="flex items-center gap-1">
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7 hover:bg-white/[0.06]" onClick={onSelectAll}>
                <CheckSquare className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Select all</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7 hover:bg-white/[0.06]" onClick={onDeselectAll}>
                <Square className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Deselect all</TooltipContent>
          </Tooltip>
        </TooltipProvider>

        <div className="w-px h-4 bg-white/[0.1] mx-1" />

        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7 hover:bg-white/[0.06]" onClick={onSortByPriority}>
                <ArrowUpDown className="w-3.5 h-3.5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Sort by priority</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon" className="h-7 w-7 hover:bg-red-500/10 hover:text-red-400" onClick={onClearAll}>
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
  const [leftPanelWidth, setLeftPanelWidth] = useState(50);
  const [isResizing, setIsResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const planArtifact = useIdeationStore((state) => state.planArtifact);
  const ideationSettings = useIdeationStore((state) => state.ideationSettings);
  const fetchPlanArtifact = useIdeationStore((state) => state.fetchPlanArtifact);
  const showSyncNotification = useIdeationStore((state) => state.showSyncNotification);
  const syncNotification = useIdeationStore((state) => state.syncNotification);
  const dismissSyncNotification = useIdeationStore((state) => state.dismissSyncNotification);

  useEffect(() => {
    if (session?.planArtifactId) {
      fetchPlanArtifact(session.planArtifactId);
    }
  }, [session?.planArtifactId, fetchPlanArtifact]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      unlisten = await listen<{ artifact_id: string; proposal_ids: string[] }>(
        "plan:proposals_may_need_update",
        (event) => {
          const affectedProposals = proposals.filter((p) => event.payload.proposal_ids.includes(p.id));
          const previousStates: Record<string, unknown> = {};
          affectedProposals.forEach((p) => { previousStates[p.id] = { ...p }; });

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
    return () => { if (unlisten) unlisten(); };
  }, [proposals, showSyncNotification]);

  useEffect(() => {
    if (messagesEndRef.current?.scrollIntoView) {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages]);

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
      setLeftPanelWidth(Math.max(30, Math.min(70, newWidth)));
    };

    const handleMouseUp = () => setIsResizing(false);

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);

    return () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isResizing]);

  const handleArchive = useCallback(() => {
    if (session) onArchiveSession(session.id);
  }, [session, onArchiveSession]);

  const handleApply = useCallback((targetColumn: string) => {
    if (!session) return;
    const selectedProposals = proposals.filter((p) => p.selected);
    onApply({
      sessionId: session.id,
      proposalIds: selectedProposals.map((p) => p.id),
      targetColumn,
      preserveDependencies: true,
    });
  }, [session, proposals, onApply]);

  const handleSelectAll = useCallback(() => {
    proposals.forEach((p) => { if (!p.selected) onSelectProposal(p.id); });
  }, [proposals, onSelectProposal]);

  const handleDeselectAll = useCallback(() => {
    proposals.forEach((p) => { if (p.selected) onSelectProposal(p.id); });
  }, [proposals, onSelectProposal]);

  const handleSortByPriority = useCallback(() => {
    const sorted = [...proposals].sort((a, b) => b.priorityScore - a.priorityScore);
    onReorderProposals(sorted.map((p) => p.id));
  }, [proposals, onReorderProposals]);

  const handleClearAll = useCallback(() => {
    proposals.forEach((p) => onRemoveProposal(p.id));
  }, [proposals, onRemoveProposal]);

  const [highlightedProposalIds, setHighlightedProposalIds] = useState<Set<string>>(new Set());
  const [planHistoryDialog, setPlanHistoryDialog] = useState<{ isOpen: boolean; artifactId: string; version: number } | null>(null);

  const handleViewHistoricalPlan = useCallback((artifactId: string, version: number) => {
    setPlanHistoryDialog({ isOpen: true, artifactId, version });
  }, []);

  const handleClosePlanHistoryDialog = useCallback(() => setPlanHistoryDialog(null), []);

  const handleReviewSync = useCallback(() => {
    if (syncNotification) {
      setHighlightedProposalIds(new Set(syncNotification.proposalIds));
      setTimeout(() => setHighlightedProposalIds(new Set()), 5000);
    }
  }, [syncNotification]);

  const handleUndoSync = useCallback(() => {
    if (!syncNotification) return;
    console.log("Undo sync - restoring proposals:", syncNotification.previousStates);
    dismissSyncNotification();
    setHighlightedProposalIds(new Set());
  }, [syncNotification, dismissSyncNotification]);

  const handleDismissSync = useCallback(() => {
    dismissSyncNotification();
    setHighlightedProposalIds(new Set());
  }, [dismissSyncNotification]);

  const [importStatus, setImportStatus] = useState<{ type: "success" | "error"; message: string } | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleImportPlan = useCallback(() => fileInputRef.current?.click(), []);

  const handleFileSelected = useCallback(async (event: React.ChangeEvent<HTMLInputElement>) => {
    if (!session) return;
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      const content = await file.text();
      const title = file.name.replace(/\.md$/, "").replace(/_/g, " ");

      const apiResponse = await fetch("http://localhost:3847/api/create_plan_artifact", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: session.id, title, content }),
      });

      if (!apiResponse.ok) throw new Error("Failed to import plan");

      const data = await apiResponse.json();
      if (data.id) {
        await fetchPlanArtifact(data.id);
        setImportStatus({ type: "success", message: `Plan "${title}" imported successfully` });
        setTimeout(() => setImportStatus(null), 5000);
      }
    } catch (error) {
      console.error("Plan import error:", error);
      setImportStatus({ type: "error", message: error instanceof Error ? error.message : "Failed to import plan" });
      setTimeout(() => setImportStatus(null), 5000);
    } finally {
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  }, [session, fetchPlanArtifact]);

  const selectedCount = proposals.filter((p) => p.selected).length;
  const canApply = selectedCount > 0 && !isLoading;

  const sortedProposals = useMemo(() => [...proposals].sort((a, b) => a.sortOrder - b.sortOrder), [proposals]);

  const groupedMessages = useMemo(() => {
    return messages.map((msg, index) => {
      const prevMsg = messages[index - 1];
      const nextMsg = messages[index + 1];
      return { ...msg, isFirstInGroup: !prevMsg || prevMsg.role !== msg.role, isLastInGroup: !nextMsg || nextMsg.role !== msg.role };
    });
  }, [messages]);

  const activeSessions = useMemo(() => sessions.filter((s) => s.status === "active"), [sessions]);

  return (
    <>
      <style>{animationStyles}</style>
      <div
        ref={containerRef}
        data-testid="ideation-view"
        className="flex h-full relative bg-[#050505]"
        role="main"
      >
        {/* Session Browser Sidebar */}
        <SessionBrowser
          sessions={activeSessions}
          currentSessionId={session?.id ?? null}
          onSelectSession={onSelectSession}
          onNewSession={onNewSession}
        />

        {/* Main Content */}
        {!session ? (
          <StartSessionPanel onNewSession={onNewSession} />
        ) : (
          <div className="flex flex-col flex-1 overflow-hidden">
            {/* Header */}
            <header
              data-testid="ideation-header"
              className="flex items-center justify-between h-14 px-6 border-b border-white/[0.06] bg-black/40 backdrop-blur-xl"
            >
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-[#ff6b35]/20 to-[#ff6b35]/5 flex items-center justify-center border border-[#ff6b35]/20">
                  <Sparkles className="w-4 h-4 text-[#ff6b35]" />
                </div>
                <div>
                  <h1 className="text-sm font-semibold text-[var(--text-primary)] tracking-tight">
                    {session.title || "New Session"}
                  </h1>
                  <p className="text-[11px] text-[var(--text-muted)]">
                    {messages.length} messages · {proposals.length} proposals
                  </p>
                </div>
              </div>
              <Button variant="ghost" onClick={handleArchive} className="gap-2 text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-white/[0.06]">
                <Archive className="w-4 h-4" />
                Archive
              </Button>
            </header>

            {/* Split Layout */}
            <div data-testid="ideation-main-content" className="flex flex-1 overflow-hidden">
              {/* Conversation Panel */}
              <div
                data-testid="conversation-panel"
                className="flex flex-col border-r border-white/[0.06] bg-gradient-to-b from-black/20 to-transparent"
                style={{ width: `${leftPanelWidth}%`, minWidth: "360px" }}
              >
                {/* Panel Header */}
                <div className="flex items-center gap-2 px-5 py-3 h-12 border-b border-white/[0.06] bg-black/20">
                  <MessageSquare className="w-4 h-4 text-[var(--text-muted)]" />
                  <h2 className="text-sm font-medium text-[var(--text-primary)]">Conversation</h2>
                </div>

                {/* Messages */}
                <div className="flex-1 overflow-y-auto p-5">
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
                <div className="border-t border-white/[0.06] bg-black/30 p-4">
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
                className={cn("w-1 cursor-ew-resize relative group", isResizing && "bg-[#ff6b35]/50")}
                onMouseDown={handleResizeStart}
              >
                <div className={cn(
                  "absolute top-0 bottom-0 left-1/2 -translate-x-1/2 w-px transition-all duration-150",
                  isResizing
                    ? "bg-[#ff6b35] shadow-[0_0_12px_rgba(255,107,53,0.5)]"
                    : "bg-white/[0.06] group-hover:bg-[#ff6b35]/60 group-hover:shadow-[0_0_8px_rgba(255,107,53,0.3)]"
                )} />
              </div>

              {/* Proposals Panel */}
              <div
                data-testid="proposals-panel"
                className="flex flex-col flex-1 bg-gradient-to-b from-black/10 to-transparent"
                style={{ minWidth: "360px" }}
              >
                {/* Panel Header */}
                <div className="flex items-center justify-between px-5 py-3 h-12 border-b border-white/[0.06] bg-black/20">
                  <div className="flex items-center gap-2">
                    <ListTodo className="w-4 h-4 text-[var(--text-muted)]" />
                    <h2 className="text-sm font-medium text-[var(--text-primary)]">Task Proposals</h2>
                  </div>
                  <span className="px-2 py-0.5 rounded-md text-[11px] font-medium bg-white/[0.05] text-[var(--text-muted)] border border-white/[0.06]">
                    {proposals.length}
                  </span>
                </div>

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

                {/* Proposals List */}
                <div className="flex-1 overflow-y-auto p-4">
                  {importStatus && (
                    <div className={cn(
                      "mb-4 p-4 rounded-xl border",
                      importStatus.type === "success"
                        ? "bg-emerald-500/10 border-emerald-500/30"
                        : "bg-red-500/10 border-red-500/30"
                    )}>
                      <div className="flex items-center justify-between">
                        <p className="text-sm font-medium text-[var(--text-primary)]">{importStatus.message}</p>
                        <Button variant="ghost" size="icon" onClick={() => setImportStatus(null)} className="h-7 w-7">×</Button>
                      </div>
                    </div>
                  )}

                  {syncNotification && (
                    <ProactiveSyncNotificationBanner
                      notification={syncNotification}
                      onDismiss={handleDismissSync}
                      onReview={handleReviewSync}
                      onUndo={handleUndoSync}
                    />
                  )}

                  {!planArtifact && proposals.length > 0 && (
                    <Button variant="outline" onClick={handleImportPlan} className="w-full mb-4 gap-2 border-white/[0.1] hover:border-white/[0.2] hover:bg-white/[0.03]" data-testid="import-plan-button">
                      <Upload className="w-4 h-4" />
                      Import Implementation Plan
                    </Button>
                  )}

                  {planArtifact && (
                    <div className="mb-4">
                      <PlanDisplay
                        plan={planArtifact}
                        showApprove={ideationSettings?.requirePlanApproval ?? false}
                        linkedProposalsCount={proposals.filter((p) => p.planArtifactId === planArtifact.id).length}
                        onEdit={() => console.log("Edit plan:", planArtifact.id)}
                      />
                    </div>
                  )}

                  {!planArtifact && ideationSettings?.planMode === "required" && proposals.length === 0 && (
                    <div className="flex flex-col items-center justify-center h-full p-8">
                      <div className="relative">
                        <div className="absolute inset-0 bg-[#ff6b35]/5 rounded-3xl blur-2xl" />
                        <div className="relative p-8 rounded-2xl bg-gradient-to-br from-white/[0.03] to-transparent border border-white/[0.06] text-center">
                          <Loader2 className="w-10 h-10 mx-auto mb-4 text-[#ff6b35] animate-spin" />
                          <p className="font-medium text-[var(--text-secondary)]">Waiting for implementation plan...</p>
                          <p className="text-sm text-[var(--text-muted)] mt-1">The orchestrator will create a plan first</p>
                        </div>
                      </div>
                    </div>
                  )}

                  {proposals.length === 0 && !(!planArtifact && ideationSettings?.planMode === "required") && <ProposalsEmptyState />}

                  {proposals.length > 0 && (
                    <div className="space-y-3">
                      {sortedProposals.map((proposal, index) => (
                        <div key={proposal.id} style={{ animationDelay: `${index * 50}ms` }}>
                          <ProposalCard
                            proposal={proposal}
                            onSelect={onSelectProposal}
                            onEdit={onEditProposal}
                            onRemove={onRemoveProposal}
                            isHighlighted={highlightedProposalIds.has(proposal.id)}
                            currentPlanVersion={planArtifact?.metadata.version ?? undefined}
                            onViewHistoricalPlan={handleViewHistoricalPlan}
                          />
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                {/* Apply Section */}
                <div
                  data-testid="apply-section"
                  className="flex items-center justify-between px-5 py-4 border-t border-white/[0.06] bg-black/30"
                >
                  <span className="text-sm text-[var(--text-muted)]">
                    <span className="text-[var(--text-primary)] font-medium">{selectedCount}</span> selected
                  </span>

                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        disabled={!canApply}
                        className={cn(
                          "gap-2",
                          canApply && "bg-gradient-to-r from-[#ff6b35] to-[#e55a2b] hover:from-[#ff7a4a] hover:to-[#ff6b35] shadow-lg shadow-[#ff6b35]/20"
                        )}
                      >
                        Apply to
                        <ChevronDown className="w-4 h-4" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end" className="bg-[#1a1a1a] border-white/[0.1]">
                      <DropdownMenuItem onClick={() => handleApply("draft")} className="hover:bg-white/[0.06]">
                        <FileEdit className="w-4 h-4 mr-2" /> Draft
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => handleApply("backlog")} className="hover:bg-white/[0.06]">
                        <Inbox className="w-4 h-4 mr-2" /> Backlog
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={() => handleApply("todo")} className="hover:bg-white/[0.06]">
                        <ListTodo className="w-4 h-4 mr-2" /> Todo
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </div>
            </div>
          </div>
        )}

        {planHistoryDialog && (
          <PlanHistoryDialog
            isOpen={planHistoryDialog.isOpen}
            onClose={handleClosePlanHistoryDialog}
            artifactId={planHistoryDialog.artifactId}
            version={planHistoryDialog.version}
          />
        )}

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
