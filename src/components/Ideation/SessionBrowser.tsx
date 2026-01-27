/**
 * SessionBrowser - Sidebar for browsing ideation sessions
 *
 * Shows list of sessions with:
 * - New session button
 * - Session cards with title, timestamp
 * - Active session highlighting
 */

import { useMemo } from "react";
import { Button } from "@/components/ui/button";
import {
  MessageSquare,
  Plus,
  Clock,
  ArrowRight,
  Layers,
  Sparkles,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { IdeationSession } from "@/types/ideation";

// ============================================================================
// Helpers
// ============================================================================

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

// ============================================================================
// Types
// ============================================================================

interface SessionBrowserProps {
  sessions: IdeationSession[];
  currentSessionId: string | null;
  onSelectSession: (sessionId: string) => void;
  onNewSession: () => void;
}

// ============================================================================
// Component
// ============================================================================

export function SessionBrowser({
  sessions,
  currentSessionId,
  onSelectSession,
  onNewSession,
}: SessionBrowserProps) {
  const sortedSessions = useMemo(
    () => [...sessions].sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()),
    [sessions]
  );

  return (
    <div
      data-testid="session-browser"
      className="flex flex-col h-full border-r border-white/[0.06]"
      style={{
        width: "260px",
        minWidth: "260px",
        flexShrink: 0,
        background: "rgba(10,10,10,0.95)",
        backdropFilter: "blur(20px)",
        WebkitBackdropFilter: "blur(20px)",
      }}
    >
      {/* Header */}
      <div className="px-3 py-3 border-b border-white/[0.06]">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <div
              className="w-7 h-7 rounded-lg flex items-center justify-center"
              style={{
                background: "rgba(255,107,53,0.1)",
                border: "1px solid rgba(255,107,53,0.2)",
              }}
            >
              <Layers className="w-3.5 h-3.5 text-[#ff6b35]" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-[var(--text-primary)] tracking-tight">Sessions</h2>
              <p className="text-[10px] text-[var(--text-muted)]">{sessions.length} total</p>
            </div>
          </div>
        </div>

        {/* New Session Button */}
        <Button
          onClick={onNewSession}
          size="sm"
          className="w-full h-8 text-xs bg-[#ff6b35] hover:bg-[#ff7a4d] text-white font-medium border-0 transition-all duration-180"
          style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.15)" }}
        >
          <Plus className="w-3.5 h-3.5 mr-1.5" />
          New Session
        </Button>
      </div>

      {/* Session List */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1">
        {sortedSessions.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full p-4 text-center">
            <div className="w-9 h-9 rounded-lg bg-white/[0.03] flex items-center justify-center mb-2 border border-white/[0.06]">
              <Sparkles className="w-4 h-4 text-[var(--text-muted)]" />
            </div>
            <p className="text-xs text-[var(--text-muted)]">No sessions yet</p>
            <p className="text-[10px] text-[var(--text-muted)] mt-0.5">Start your first brainstorm</p>
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
                  "session-card-enter w-full p-2.5 rounded-lg text-left transition-all duration-180",
                  "border border-transparent",
                  "hover:bg-white/[0.03] hover:border-white/[0.06]"
                )}
                style={{
                  animationDelay: `${index * 50}ms`,
                  ...(isSelected && {
                    background: "rgba(255,107,53,0.08)",
                    borderColor: "rgba(255,107,53,0.25)",
                  }),
                }}
              >
                <div className="flex items-start gap-2.5">
                  {/* Session indicator */}
                  <div
                    className="w-7 h-7 rounded-md flex items-center justify-center flex-shrink-0 transition-colors"
                    style={{
                      background: isSelected ? "rgba(255,107,53,0.15)" : "rgba(255,255,255,0.03)",
                      border: isSelected ? "1px solid rgba(255,107,53,0.25)" : "1px solid rgba(255,255,255,0.06)",
                    }}
                  >
                    <MessageSquare className={cn("w-3.5 h-3.5", isSelected ? "text-[#ff6b35]" : "text-[var(--text-muted)]")} />
                  </div>

                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-1.5 mb-0.5">
                      <span className={cn(
                        "text-xs font-medium truncate",
                        isSelected ? "text-[var(--text-primary)]" : "text-[var(--text-secondary)]"
                      )}>
                        {session.title || "Untitled Session"}
                      </span>
                      {isSelected && (
                        <span className="w-1 h-1 rounded-full bg-[#ff6b35] flex-shrink-0" />
                      )}
                    </div>
                    <div className="flex items-center gap-1 text-[10px] text-[var(--text-muted)]">
                      <Clock className="w-2.5 h-2.5" />
                      <span>{formatRelativeTime(session.updatedAt)}</span>
                    </div>
                  </div>

                  {/* Arrow indicator */}
                  <ArrowRight className={cn(
                    "w-3.5 h-3.5 flex-shrink-0 transition-all duration-200",
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
