/**
 * EmptyStates - Empty state components for Ideation view
 *
 * - ConversationEmptyState: Shown when no messages in conversation
 * - WaitingForCapacityState: Shown when session has pending_initial_prompt set (waiting for slot)
 */

import { Clock, MessageSquareText, Settings } from "lucide-react";
import { useExecutionStatus } from "@/hooks/useExecutionControl";
import { useUiStore } from "@/stores/uiStore";
import { withAlpha } from "@/lib/theme-colors";

// ============================================================================
// Conversation Empty State
// ============================================================================

// ============================================================================
// Waiting for Capacity State
// ============================================================================

interface WaitingForCapacityStateProps {
  /** The queued message text to display (pending_initial_prompt). */
  pendingInitialPrompt?: string | null;
  /** Project ID for per-project capacity scoping. */
  projectId?: string;
}

export function WaitingForCapacityState({ pendingInitialPrompt, projectId }: WaitingForCapacityStateProps) {
  const { data, isLoading, isError } = useExecutionStatus(projectId);
  const openModal = useUiStore((s) => s.openModal);

  const hasCapacityData = !isLoading && !isError && data != null;

  return (
    <div className="flex flex-col items-center justify-center h-full p-6">
      <div className="text-center max-w-[320px]">
        {/* Pulsing amber clock icon */}
        <div className="flex items-center justify-center mb-6">
          <div
            className="w-14 h-14 rounded-full flex items-center justify-center animate-pulse"
            style={{
              background: "var(--status-warning-muted)",
              border: "1px solid var(--status-warning-border)",
            }}
          >
            <Clock className="w-6 h-6" style={{ color: "var(--status-warning)" }} />
          </div>
        </div>

        <h3 className="text-base font-semibold mb-2 tracking-tight" style={{ color: "var(--text-primary)" }}>
          Waiting for capacity
        </h3>

        {hasCapacityData ? (
          <p className="text-sm leading-relaxed" style={{ color: "var(--text-secondary)" }}>
            Ideation capacity:{" "}
            <span style={{ color: "var(--text-primary)" }}>
              {data.ideationActive}/{data.ideationMaxProject} {data.ideationMaxProject === 1 ? "slot" : "slots"} active in this project.
            </span>
            {data.ideationWaiting > 0 && (
              <>{" "}{data.ideationWaiting} {data.ideationWaiting === 1 ? "session" : "sessions"} waiting to start.</>
            )}{" "}
            This session will start automatically when a slot opens.
          </p>
        ) : (
          <p className="text-sm leading-relaxed" style={{ color: "var(--text-secondary)" }}>
            All ideation slots are in use. This session will start automatically when a slot opens.
          </p>
        )}

        {/* Settings navigation link */}
        <button
          onClick={() => openModal("settings", { section: "ideation-workflow" })}
          className="mt-3 inline-flex items-center gap-1 text-xs transition-colors"
          style={{ color: "var(--status-warning)" }}
        >
          <Settings className="w-3 h-3" />
          Adjust limits in Settings →
        </button>

        {/* Queued message preview */}
        {pendingInitialPrompt && (
          <div
            className="mt-5 text-left rounded-md px-3 py-2.5"
            style={{
              background: "var(--overlay-faint)",
              border: "1px solid var(--overlay-weak)",
              borderLeft: `2px solid ${withAlpha("var(--status-warning)", 50)}`,
            }}
          >
            <p className="text-xs mb-1 font-medium" style={{ color: "var(--text-muted)" }}>
              Your queued message
            </p>
            <p
              className="text-sm leading-relaxed line-clamp-4"
              style={{ color: "var(--text-secondary)" }}
            >
              {pendingInitialPrompt}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

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
              background: "var(--overlay-weak)",
              border: "1px solid var(--overlay-faint)",
            }}
          />
          <div
            className="w-10 h-10 rounded-full flex items-center justify-center opacity-50"
            style={{
              background: "var(--overlay-weak)",
              border: "1px solid var(--overlay-weak)",
            }}
          />
          <div
            className="w-14 h-14 rounded-full flex items-center justify-center"
            style={{
              background: withAlpha("var(--accent-primary)", 12),
              border: "1px solid var(--accent-border)",
            }}
          >
            <MessageSquareText className="w-6 h-6" style={{ color: "var(--accent-primary)" }} />
          </div>
          <div
            className="w-10 h-10 rounded-full flex items-center justify-center opacity-50"
            style={{
              background: "var(--overlay-weak)",
              border: "1px solid var(--overlay-weak)",
            }}
          />
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center opacity-30"
            style={{
              background: "var(--overlay-weak)",
              border: "1px solid var(--overlay-faint)",
            }}
          />
        </div>

        <h3 className="text-base font-semibold mb-2 tracking-tight" style={{ color: "var(--text-primary)" }}>
          Start the conversation
        </h3>
        <p className="text-sm leading-relaxed" style={{ color: "var(--text-secondary)" }}>
          Describe your ideas and I'll help create task proposals
        </p>

        {/* Hint arrow pointing down to input */}
        <div className="mt-6 flex justify-center">
          <div
            className="w-8 h-8 rounded-full flex items-center justify-center animate-bounce"
            style={{
              background: withAlpha("var(--accent-primary)", 10),
              border: "1px solid var(--accent-border)",
            }}
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" style={{ color: "var(--accent-primary)" }}>
              <path d="M7 2v8m0 0l-3-3m3 3l3-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
          </div>
        </div>
      </div>
    </div>
  );
}


