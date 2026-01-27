/**
 * EmptyStates - Empty state components for Ideation view
 *
 * - ConversationEmptyState: Shown when no messages in conversation
 * - ProposalsEmptyState: Shown when no proposals exist
 */

import { MessageSquareText, Lightbulb } from "lucide-react";

// ============================================================================
// Conversation Empty State
// ============================================================================

export function ConversationEmptyState() {
  return (
    <div className="flex flex-col items-center justify-center h-full p-6">
      {/* Minimal, flowing design for conversation */}
      <div className="text-center max-w-[280px]">
        {/* Animated message bubbles */}
        <div className="flex items-end justify-center gap-2 mb-6">
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center opacity-30"
            style={{
              background: "linear-gradient(135deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.03) 100%)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          />
          <div
            className="w-10 h-10 rounded-full flex items-center justify-center opacity-50"
            style={{
              background: "linear-gradient(135deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0.04) 100%)",
              border: "1px solid rgba(255,255,255,0.08)",
            }}
          />
          <div
            className="w-14 h-14 rounded-full flex items-center justify-center"
            style={{
              background: "linear-gradient(135deg, rgba(255,107,53,0.15) 0%, rgba(255,107,53,0.05) 100%)",
              border: "1px solid rgba(255,107,53,0.25)",
              boxShadow: "0 0 30px rgba(255,107,53,0.1)",
            }}
          >
            <MessageSquareText className="w-6 h-6 text-[#ff6b35]" />
          </div>
          <div
            className="w-10 h-10 rounded-full flex items-center justify-center opacity-50"
            style={{
              background: "linear-gradient(135deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0.04) 100%)",
              border: "1px solid rgba(255,255,255,0.08)",
            }}
          />
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center opacity-30"
            style={{
              background: "linear-gradient(135deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.03) 100%)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          />
        </div>

        <h3 className="text-base font-semibold text-[var(--text-primary)] mb-2 tracking-tight">
          Start the conversation
        </h3>
        <p className="text-sm text-[var(--text-secondary)] leading-relaxed">
          Describe your ideas and I'll help create task proposals
        </p>

        {/* Hint arrow pointing down to input */}
        <div className="mt-6 flex justify-center">
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center animate-bounce"
            style={{
              background: "rgba(255,107,53,0.1)",
              border: "1px solid rgba(255,107,53,0.2)",
            }}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" className="text-[#ff6b35]">
              <path d="M7 2v8m0 0l-3-3m3 3l3-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </div>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Proposals Empty State
// ============================================================================

export function ProposalsEmptyState() {
  return (
    <div data-testid="proposals-empty-state" className="flex flex-col items-center pt-[20%] h-full p-6">
      {/* Structured, task-list oriented design for proposals */}
      <div className="w-full max-w-[280px]">
        {/* Mock task cards with dashed borders - suggests awaiting content */}
        <div className="space-y-2 mb-5">
          {[0.4, 0.25, 0.15].map((opacity, i) => (
            <div
              key={i}
              className="flex items-center gap-3 p-3 rounded-lg"
              style={{
                opacity,
                border: "1.5px dashed rgba(255,107,53,0.25)",
                background: "rgba(255,107,53,0.02)",
              }}
            >
              <div
                className="w-4 h-4 rounded border-[1.5px] border-dashed flex-shrink-0"
                style={{ borderColor: "rgba(255,107,53,0.4)" }}
              />
              <div
                className="h-2 rounded-full flex-1"
                style={{
                  background: "linear-gradient(90deg, rgba(255,255,255,0.08) 0%, rgba(255,255,255,0.03) 100%)",
                  maxWidth: `${70 - i * 15}%`,
                }}
              />
            </div>
          ))}
        </div>

        {/* Central icon with glow */}
        <div className="flex justify-center mb-4">
          <div
            className="w-12 h-12 rounded-xl flex items-center justify-center relative"
            style={{
              background: "linear-gradient(135deg, rgba(251,191,36,0.15) 0%, rgba(251,191,36,0.05) 100%)",
              border: "1px solid rgba(251,191,36,0.25)",
              boxShadow: "0 0 24px rgba(251,191,36,0.1)",
            }}
          >
            <Lightbulb className="w-5 h-5 text-amber-400" />
          </div>
        </div>

        {/* Text content */}
        <div className="text-center">
          <h3 className="text-sm font-semibold text-[var(--text-primary)] mb-1.5 tracking-tight">
            No proposals yet
          </h3>
          <p className="text-xs text-[var(--text-secondary)] leading-relaxed">
            Ideas from the conversation will appear here as task proposals
          </p>
        </div>

        {/* Visual connection hint - arrow pointing left to chat */}
        <div className="flex justify-center mt-5">
          <div
            className="flex items-center gap-2 px-3 py-1.5 rounded-full"
            style={{
              background: "rgba(255,255,255,0.03)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" className="text-[var(--text-muted)]">
              <path d="M12 7H2m0 0l3-3m-3 3l3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
            <span className="text-[10px] text-[var(--text-muted)] uppercase tracking-wider">From chat</span>
          </div>
        </div>
      </div>
    </div>
  );
}
