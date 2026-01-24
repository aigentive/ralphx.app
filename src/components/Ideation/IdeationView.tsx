/**
 * IdeationView - Main ideation interface with split layout
 *
 * Features:
 * - Split layout: Conversation (left) + Proposals (right)
 * - Header with session title, New Session, Archive buttons
 * - Conversation panel with message history
 * - Proposals panel with ProposalList
 * - Apply dropdown for target column selection
 * - Responsive layout (stacks on mobile)
 */

import { useState, useCallback, useRef, useEffect } from "react";
import type {
  IdeationSession,
  TaskProposal,
  ChatMessage as ChatMessageType,
  ApplyProposalsInput,
} from "@/types/ideation";
import { ChatMessage } from "@/components/Chat/ChatMessage";
import { ChatInput } from "@/components/Chat/ChatInput";
import { ProposalList } from "./ProposalList";

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

const TARGET_COLUMNS = [
  { value: "draft", label: "Draft" },
  { value: "backlog", label: "Backlog" },
  { value: "todo", label: "Todo" },
];

// ============================================================================
// Component
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
  const [applyDropdownOpen, setApplyDropdownOpen] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    if (messagesEndRef.current && typeof messagesEndRef.current.scrollIntoView === "function") {
      messagesEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages]);

  const handleSendMessage = useCallback(
    (content: string) => {
      onSendMessage(content);
    },
    [onSendMessage]
  );

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
      setApplyDropdownOpen(false);
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

  // No session state
  if (!session) {
    return (
      <div
        data-testid="ideation-view"
        className="flex flex-col items-center justify-center h-full p-8"
        style={{ backgroundColor: "var(--bg-base)" }}
        role="main"
      >
        <div className="text-center">
          <h2
            className="text-xl font-semibold mb-2"
            style={{ color: "var(--text-primary)" }}
          >
            Start a new ideation session
          </h2>
          <p
            className="text-sm mb-4"
            style={{ color: "var(--text-secondary)" }}
          >
            Brainstorm ideas and create task proposals
          </p>
          <button
            onClick={onNewSession}
            className="px-4 py-2 rounded text-sm font-medium transition-colors"
            style={{
              backgroundColor: "var(--accent-primary)",
              color: "var(--bg-base)",
            }}
          >
            Start Session
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      data-testid="ideation-view"
      className="flex flex-col h-full"
      style={{ backgroundColor: "var(--bg-base)" }}
      role="main"
    >
      {/* Loading overlay */}
      {isLoading && (
        <div
          data-testid="ideation-loading"
          className="absolute inset-0 flex items-center justify-center z-10"
          style={{ backgroundColor: "rgba(0, 0, 0, 0.3)" }}
        >
          <div
            className="px-4 py-2 rounded"
            style={{ backgroundColor: "var(--bg-elevated)", color: "var(--text-primary)" }}
          >
            Loading...
          </div>
        </div>
      )}

      {/* Header */}
      <header
        data-testid="ideation-header"
        className="flex items-center justify-between px-4 py-3 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        <h1
          className="text-lg font-semibold truncate"
          style={{ color: "var(--text-primary)" }}
        >
          {session.title ?? "New Session"}
        </h1>
        <div className="flex items-center gap-2">
          <button
            onClick={onNewSession}
            className="px-3 py-1.5 rounded text-sm font-medium transition-colors hover:opacity-80"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-primary)",
            }}
          >
            New Session
          </button>
          <button
            onClick={handleArchive}
            className="px-3 py-1.5 rounded text-sm font-medium transition-colors hover:opacity-80"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-secondary)",
            }}
          >
            Archive
          </button>
        </div>
      </header>

      {/* Main content - split layout */}
      <div
        data-testid="ideation-main-content"
        className="flex flex-col lg:flex-row flex-1 overflow-hidden"
      >
        {/* Conversation Panel (left) */}
        <div
          data-testid="conversation-panel"
          className="flex flex-col flex-1 lg:w-1/2 border-r lg:border-r"
          style={{
            backgroundColor: "var(--bg-surface)",
            borderColor: "var(--border-subtle)",
          }}
        >
          <div
            className="px-4 py-2 border-b"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <h2
              className="text-sm font-medium"
              style={{ color: "var(--text-primary)" }}
            >
              Conversation
            </h2>
          </div>

          {/* Messages */}
          <div className="flex-1 overflow-y-auto p-4">
            {messages.length === 0 ? (
              <div className="flex items-center justify-center h-full">
                <p
                  className="text-sm italic"
                  style={{ color: "var(--text-muted)" }}
                >
                  Start the conversation by sending a message
                </p>
              </div>
            ) : (
              <div className="space-y-1">
                {messages.map((message) => (
                  <ChatMessage key={message.id} message={message} compact />
                ))}
                <div ref={messagesEndRef} />
              </div>
            )}
          </div>

          {/* Input */}
          <div
            className="px-4 py-3 border-t"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <ChatInput
              onSend={handleSendMessage}
              isSending={isLoading}
              placeholder="Send a message..."
            />
          </div>
        </div>

        {/* Proposals Panel (right) */}
        <div
          data-testid="proposals-panel"
          className="flex flex-col flex-1 lg:w-1/2"
          style={{ backgroundColor: "var(--bg-surface)" }}
        >
          <div
            className="flex items-center justify-between px-4 py-2 border-b"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <h2
              className="text-sm font-medium"
              style={{ color: "var(--text-primary)" }}
            >
              Task Proposals
            </h2>
            <span
              className="text-xs"
              style={{ color: "var(--text-muted)" }}
            >
              {proposals.length} proposals
            </span>
          </div>

          {/* Proposals List */}
          <div className="flex-1 overflow-y-auto p-4">
            {proposals.length === 0 ? (
              <div className="flex items-center justify-center h-full">
                <p
                  className="text-sm italic"
                  style={{ color: "var(--text-muted)" }}
                >
                  No proposals yet
                </p>
              </div>
            ) : (
              <ProposalList
                proposals={proposals}
                onSelect={onSelectProposal}
                onEdit={onEditProposal}
                onRemove={onRemoveProposal}
                onReorder={onReorderProposals}
                onSelectAll={handleSelectAll}
                onDeselectAll={handleDeselectAll}
                onSortByPriority={handleSortByPriority}
                onClearAll={handleClearAll}
              />
            )}
          </div>

          {/* Apply Section */}
          <div
            data-testid="apply-section"
            className="flex items-center justify-between px-4 py-3 border-t"
            style={{ borderColor: "var(--border-subtle)" }}
          >
            <span
              className="text-sm"
              style={{ color: "var(--text-secondary)" }}
            >
              {selectedCount} selected
            </span>

            <div className="relative">
              <button
                onClick={() => setApplyDropdownOpen(!applyDropdownOpen)}
                disabled={!canApply}
                className="px-4 py-2 rounded text-sm font-medium transition-colors flex items-center gap-1 disabled:opacity-50 disabled:cursor-not-allowed"
                style={{
                  backgroundColor: canApply ? "var(--accent-primary)" : "var(--bg-hover)",
                  color: canApply ? "var(--bg-base)" : "var(--text-secondary)",
                }}
              >
                Apply to
                <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
                  <path d="M3 5l3 3 3-3" stroke="currentColor" strokeWidth="1.5" fill="none" />
                </svg>
              </button>

              {applyDropdownOpen && (
                <div
                  className="absolute bottom-full right-0 mb-1 w-32 rounded shadow-lg border overflow-hidden"
                  style={{
                    backgroundColor: "var(--bg-elevated)",
                    borderColor: "var(--border-subtle)",
                  }}
                >
                  {TARGET_COLUMNS.map((col) => (
                    <button
                      key={col.value}
                      onClick={() => handleApply(col.value)}
                      className="w-full px-3 py-2 text-sm text-left hover:bg-[--bg-hover] transition-colors"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {col.label}
                    </button>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
