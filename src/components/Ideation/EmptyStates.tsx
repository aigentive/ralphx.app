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
              background: "hsla(220 10% 100% / 0.05)",
              border: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          />
          <div
            className="w-10 h-10 rounded-full flex items-center justify-center opacity-50"
            style={{
              background: "hsla(220 10% 100% / 0.06)",
              border: "1px solid hsla(220 10% 100% / 0.08)",
            }}
          />
          <div
            className="w-14 h-14 rounded-full flex items-center justify-center"
            style={{
              background: "hsla(14 100% 60% / 0.12)",
              border: "1px solid hsla(14 100% 60% / 0.25)",
            }}
          >
            <MessageSquareText className="w-6 h-6" style={{ color: "hsl(14 100% 60%)" }} />
          </div>
          <div
            className="w-10 h-10 rounded-full flex items-center justify-center opacity-50"
            style={{
              background: "hsla(220 10% 100% / 0.06)",
              border: "1px solid hsla(220 10% 100% / 0.08)",
            }}
          />
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center opacity-30"
            style={{
              background: "hsla(220 10% 100% / 0.05)",
              border: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          />
        </div>

        <h3 className="text-base font-semibold mb-2 tracking-tight" style={{ color: "hsl(220 10% 90%)" }}>
          Start the conversation
        </h3>
        <p className="text-sm leading-relaxed" style={{ color: "hsl(220 10% 60%)" }}>
          Describe your ideas and I'll help create task proposals
        </p>

        {/* Hint arrow pointing down to input */}
        <div className="mt-6 flex justify-center">
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center animate-bounce"
            style={{
              background: "hsla(14 100% 60% / 0.1)",
              border: "1px solid hsla(14 100% 60% / 0.2)",
            }}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" style={{ color: "hsl(14 100% 60%)" }}>
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
                border: "1.5px dashed hsla(14 100% 60% / 0.25)",
                background: "hsla(14 100% 60% / 0.02)",
              }}
            >
              <div
                className="w-4 h-4 rounded border-[1.5px] border-dashed flex-shrink-0"
                style={{ borderColor: "hsla(14 100% 60% / 0.4)" }}
              />
              <div
                className="h-2 rounded-full flex-1"
                style={{
                  background: "hsla(220 10% 100% / 0.06)",
                  maxWidth: `${70 - i * 15}%`,
                }}
              />
            </div>
          ))}
        </div>

        {/* Central icon - flat style */}
        <div className="flex justify-center mb-4">
          <div
            className="w-12 h-12 rounded-xl flex items-center justify-center relative"
            style={{
              background: "hsla(45 93% 50% / 0.12)",
              border: "1px solid hsla(45 93% 50% / 0.25)",
            }}
          >
            <Lightbulb className="w-5 h-5" style={{ color: "hsl(45 93% 55%)" }} />
          </div>
        </div>

        {/* Text content */}
        <div className="text-center">
          <h3 className="text-sm font-semibold mb-1.5 tracking-tight" style={{ color: "hsl(220 10% 90%)" }}>
            No proposals yet
          </h3>
          <p className="text-xs leading-relaxed" style={{ color: "hsl(220 10% 60%)" }}>
            Ideas from the conversation will appear here as task proposals
          </p>
        </div>

        {/* Visual connection hint - arrow pointing left to chat */}
        <div className="flex justify-center mt-5">
          <div
            className="flex items-center gap-2 px-3 py-1.5 rounded-full"
            style={{
              background: "hsla(220 10% 100% / 0.03)",
              border: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" style={{ color: "hsl(220 10% 50%)" }}>
              <path d="M12 7H2m0 0l3-3m-3 3l3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
            <span className="text-[10px] uppercase tracking-wider" style={{ color: "hsl(220 10% 50%)" }}>From chat</span>
          </div>
        </div>
      </div>
    </div>
  );
}
