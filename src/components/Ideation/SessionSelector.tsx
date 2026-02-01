/**
 * SessionSelector - Dropdown component for selecting and managing ideation sessions
 *
 * Features:
 * - Dropdown listing sessions for project
 * - Session status indicators (active/archived/accepted)
 * - New session button
 * - Archive action per session
 * - Keyboard navigation (Escape to close)
 * - Click outside to close
 */

import { useState, useCallback, useEffect, useRef } from "react";
import type { IdeationSession, IdeationSessionStatus } from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

interface SessionSelectorProps {
  sessions: IdeationSession[];
  currentSession: IdeationSession | null;
  onSelectSession: (sessionId: string) => void;
  onNewSession: () => void;
  onArchiveSession: (sessionId: string) => void;
  isLoading?: boolean;
}

// ============================================================================
// Status Configuration
// ============================================================================

const STATUS_CONFIG: Record<IdeationSessionStatus, { color: string; label: string }> = {
  active: { color: "var(--status-success)", label: "Active" },
  archived: { color: "var(--text-muted)", label: "Archived" },
  accepted: { color: "var(--status-info)", label: "Accepted" },
};

// ============================================================================
// Component
// ============================================================================

export function SessionSelector({
  sessions,
  currentSession,
  onSelectSession,
  onNewSession,
  onArchiveSession,
  isLoading = false,
}: SessionSelectorProps) {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close dropdown on click outside
  useEffect(() => {
    if (!isOpen) return;

    const handleMouseDown = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleMouseDown);
    return () => document.removeEventListener("mousedown", handleMouseDown);
  }, [isOpen]);

  // Close dropdown on Escape key
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setIsOpen(false);
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen]);

  const handleToggle = useCallback(() => {
    if (!isLoading) {
      setIsOpen((prev) => !prev);
    }
  }, [isLoading]);

  const handleSelectSession = useCallback(
    (sessionId: string) => {
      onSelectSession(sessionId);
      setIsOpen(false);
    },
    [onSelectSession]
  );

  const handleArchive = useCallback(
    (e: React.MouseEvent, sessionId: string) => {
      e.stopPropagation();
      onArchiveSession(sessionId);
    },
    [onArchiveSession]
  );

  const getSessionTitle = (session: IdeationSession | null) => {
    if (!session) return "Select Session";
    return session.title ?? "New Session";
  };

  return (
    <div
      ref={containerRef}
      data-testid="session-selector"
      className="relative inline-flex items-center gap-2"
      style={{ backgroundColor: "var(--bg-surface)" }}
    >
      {/* Dropdown Trigger */}
      <button
        data-testid="dropdown-trigger"
        onClick={handleToggle}
        disabled={isLoading}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        className="flex items-center gap-2 px-3 py-1.5 rounded text-sm transition-colors hover:bg-[--bg-hover] disabled:opacity-50 disabled:cursor-not-allowed"
        style={{ color: "var(--text-primary)" }}
      >
        <span
          data-testid="current-session-title"
          className="font-medium"
          style={{ color: "var(--text-primary)" }}
        >
          {getSessionTitle(currentSession)}
        </span>
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="currentColor"
          style={{ color: "var(--text-secondary)" }}
        >
          <path d="M3 5l3 3 3-3" stroke="currentColor" strokeWidth="1.5" fill="none" />
        </svg>
      </button>

      {/* Loading Indicator */}
      {isLoading && (
        <span
          data-testid="loading-indicator"
          className="text-xs"
          style={{ color: "var(--text-muted)" }}
        >
          Loading...
        </span>
      )}

      {/* New Session Button */}
      <button
        onClick={onNewSession}
        disabled={isLoading}
        className="px-3 py-1.5 rounded text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        style={{
          backgroundColor: "var(--accent-primary)",
          color: "var(--bg-base)",
        }}
      >
        New Session
      </button>

      {/* Dropdown */}
      {isOpen && (
        <div
          data-testid="session-dropdown"
          role="listbox"
          className="absolute top-full left-0 mt-1 w-64 max-h-80 overflow-y-auto rounded shadow-lg border z-50"
          style={{
            backgroundColor: "var(--bg-elevated)",
            borderColor: "var(--border-subtle)",
          }}
        >
          {sessions.length === 0 ? (
            <div
              className="px-3 py-4 text-sm text-center"
              style={{ color: "var(--text-muted)" }}
            >
              No sessions yet
            </div>
          ) : (
            sessions.map((session) => {
              const isCurrent = currentSession?.id === session.id;
              const statusConfig = STATUS_CONFIG[session.status];
              const canArchive = session.status === "active";

              return (
                <div
                  key={session.id}
                  data-testid="session-item"
                  data-current={isCurrent ? "true" : "false"}
                  role="option"
                  aria-selected={isCurrent}
                  onClick={() => handleSelectSession(session.id)}
                  className="flex items-center justify-between px-3 py-2 cursor-pointer hover:bg-[--bg-hover] transition-colors"
                  style={{
                    backgroundColor: isCurrent ? "var(--bg-hover)" : undefined,
                  }}
                >
                  <div className="flex items-center gap-2 flex-1 min-w-0">
                    {/* Status Indicator */}
                    <span
                      data-testid="status-indicator"
                      data-status={session.status}
                      className="w-2 h-2 rounded-full flex-shrink-0"
                      style={{ backgroundColor: statusConfig.color }}
                      title={statusConfig.label}
                    />
                    {/* Session Title */}
                    <span
                      className="text-sm truncate"
                      style={{ color: "var(--text-primary)" }}
                    >
                      {session.title ?? "New Session"}
                    </span>
                  </div>

                  {/* Archive Button */}
                  {canArchive && (
                    <button
                      onClick={(e) => handleArchive(e, session.id)}
                      aria-label={`Archive ${session.title ?? "New Session"}`}
                      className="p-1 rounded hover:bg-[--bg-base] transition-colors flex-shrink-0"
                      style={{ color: "var(--text-secondary)" }}
                    >
                      <svg width="14" height="14" viewBox="0 0 14 14" fill="currentColor">
                        <rect x="1" y="2" width="12" height="3" rx="0.5" stroke="currentColor" strokeWidth="1.2" fill="none" />
                        <path d="M2 5v7a1 1 0 001 1h8a1 1 0 001-1V5" stroke="currentColor" strokeWidth="1.2" fill="none" />
                        <path d="M5 8h4" stroke="currentColor" strokeWidth="1.2" />
                      </svg>
                    </button>
                  )}
                </div>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}
