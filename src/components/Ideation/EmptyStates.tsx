/**
 * EmptyStates - Empty state components for Ideation view
 *
 * - ConversationEmptyState: Shown when no messages in conversation
 */

import { MessageSquareText } from "lucide-react";

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


