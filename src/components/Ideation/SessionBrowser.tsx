/**
 * SessionBrowser - macOS Tahoe styled sidebar for ideation sessions
 *
 * Design: Native macOS sidebar with frosted glass, refined typography,
 * and smooth spring animations. Warm orange accent (#ff6b35).
 */

import { useMemo, useState, useRef, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  MessageSquare,
  Plus,
  Clock,
  Sparkles,
  MoreHorizontal,
  Pencil,
  Archive,
  Trash2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { IdeationSession } from "@/types/ideation";
import { ideationApi } from "@/api/ideation";

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
  onArchiveSession?: (sessionId: string) => void;
  onDeleteSession?: (sessionId: string) => void;
}

// ============================================================================
// Component
// ============================================================================

export function SessionBrowser({
  sessions,
  currentSessionId,
  onSelectSession,
  onNewSession,
  onArchiveSession,
  onDeleteSession,
}: SessionBrowserProps) {
  const sortedSessions = useMemo(
    () => [...sessions].sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()),
    [sessions]
  );

  const [editingSessionId, setEditingSessionId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState("");
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editingSessionId && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingSessionId]);

  const handleStartRename = (session: IdeationSession) => {
    setEditingSessionId(session.id);
    setEditingTitle(session.title || "");
  };

  const handleCancelRename = () => {
    setEditingSessionId(null);
    setEditingTitle("");
  };

  const handleConfirmRename = async (sessionId: string) => {
    const trimmedTitle = editingTitle.trim();
    if (trimmedTitle) {
      try {
        await ideationApi.sessions.updateTitle(sessionId, trimmedTitle);
      } catch (error) {
        console.error("Failed to rename session:", error);
      }
    }
    setEditingSessionId(null);
    setEditingTitle("");
  };

  const handleKeyDown = (e: React.KeyboardEvent, sessionId: string) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleConfirmRename(sessionId);
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCancelRename();
    }
  };

  return (
    <div
      data-testid="session-browser"
      className="flex flex-col h-full"
      style={{
        width: "276px",
        minWidth: "276px",
        flexShrink: 0,
      }}
    >
      {/* Floating panel inner container */}
      <div
        className="flex flex-col h-full rounded-[10px]"
        style={{
          margin: "8px",
          background: "hsla(220 10% 10% / 0.92)",
          backdropFilter: "blur(20px) saturate(180%)",
          WebkitBackdropFilter: "blur(20px) saturate(180%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          boxShadow: "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
        }}
      >
        {/* Header */}
        <div
          className="px-4 pt-4 pb-3"
          style={{
            borderBottom: "1px solid hsla(220 10% 100% / 0.04)",
          }}
        >
          {/* Title */}
          <div className="flex items-center gap-2.5 mb-4">
            <div
              className="w-8 h-8 rounded-[10px] flex items-center justify-center"
              style={{
                background: "hsla(14 100% 60% / 0.12)",
                border: "1px solid hsla(14 100% 60% / 0.2)",
              }}
            >
              <Sparkles className="w-4 h-4" style={{ color: "hsl(14 100% 60%)" }} />
            </div>
            <div>
              <h2
                className="text-[13px] font-semibold tracking-[-0.01em]"
                style={{ color: "hsl(220 10% 90%)" }}
              >
                Sessions
              </h2>
              <p
                className="text-[11px] tracking-[-0.005em]"
                style={{ color: "hsl(220 10% 50%)" }}
              >
                {sessions.length} {sessions.length === 1 ? "session" : "sessions"}
              </p>
            </div>
          </div>

          {/* New Session Button - flat Tahoe style */}
          <Button
            onClick={onNewSession}
            className="w-full h-9 text-[13px] font-medium tracking-[-0.01em] border-0 transition-colors duration-150"
            style={{
              background: "hsl(14 100% 60%)",
              color: "white",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 55%)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 60%)";
            }}
          >
            <Plus className="w-4 h-4 mr-1.5" strokeWidth={2.5} />
            New Session
          </Button>
        </div>

        {/* Session List */}
        <div className="flex-1 overflow-y-auto px-2 py-2">
          {sortedSessions.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full px-4 text-center">
              <div
                className="w-12 h-12 rounded-2xl flex items-center justify-center mb-3"
                style={{
                  background: "hsla(220 10% 100% / 0.03)",
                  border: "1px solid hsla(220 10% 100% / 0.06)",
                }}
              >
                <MessageSquare className="w-5 h-5" style={{ color: "hsl(220 10% 50%)" }} />
              </div>
              <p className="text-[13px] font-medium" style={{ color: "hsl(220 10% 70%)" }}>
                No sessions yet
              </p>
              <p className="text-[11px] mt-1" style={{ color: "hsl(220 10% 50%)" }}>
                Start your first brainstorm
              </p>
            </div>
          ) : (
          <div className="space-y-1">
            {sortedSessions.map((session) => {
              const isSelected = session.id === currentSessionId;
              const isEditing = editingSessionId === session.id;
              const isMenuOpen = openMenuId === session.id;

              return (
                <div
                  key={session.id}
                  data-testid={`session-item-${session.id}`}
                  className={cn(
                    "group relative rounded-md cursor-pointer",
                    "transition-all duration-150 ease-out"
                  )}
                  style={{
                    padding: "6px 8px",
                    background: isSelected
                      ? "hsla(14 100% 60% / 0.12)"
                      : isMenuOpen
                        ? "hsla(220 10% 100% / 0.04)"
                        : "transparent",
                    border: isSelected
                      ? "1px solid hsla(14 100% 60% / 0.2)"
                      : "1px solid transparent",
                  }}
                  onClick={() => !isEditing && onSelectSession(session.id)}
                  onMouseEnter={(e) => {
                    if (!isSelected && !isMenuOpen) {
                      e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!isSelected && !isMenuOpen) {
                      e.currentTarget.style.background = "transparent";
                    }
                  }}
                >
                  <div className="flex items-center gap-2">
                    {/* Session icon */}
                    <div
                      className="w-6 h-6 rounded-md flex items-center justify-center flex-shrink-0 transition-colors duration-150"
                      style={{
                        background: isSelected
                          ? "hsla(14 100% 60% / 0.15)"
                          : "hsla(220 10% 100% / 0.04)",
                        border: isSelected
                          ? "1px solid hsla(14 100% 60% / 0.2)"
                          : "1px solid hsla(220 10% 100% / 0.06)",
                      }}
                    >
                      <MessageSquare
                        className="w-3 h-3"
                        style={{ color: isSelected ? "hsl(14 100% 60%)" : "hsl(220 10% 50%)" }}
                      />
                    </div>

                    {/* Content */}
                    <div className="flex-1 min-w-0">
                      {isEditing ? (
                        <Input
                          ref={inputRef}
                          value={editingTitle}
                          onChange={(e) => setEditingTitle(e.target.value)}
                          onKeyDown={(e) => handleKeyDown(e, session.id)}
                          onBlur={() => handleConfirmRename(session.id)}
                          className="h-6 text-[13px] px-2 py-0 rounded-md"
                          style={{
                            background: "hsl(220 10% 12%)",
                            border: "1px solid hsla(220 10% 100% / 0.1)",
                          }}
                          onClick={(e) => e.stopPropagation()}
                        />
                      ) : (
                        <>
                          <div className="flex items-center gap-1.5">
                            <span
                              className={cn(
                                "text-[12px] font-medium truncate tracking-[-0.01em]",
                                "transition-colors duration-150"
                              )}
                              style={{
                                color: isSelected ? "hsl(220 10% 90%)" : "hsl(220 10% 70%)",
                              }}
                            >
                              {session.title || "Untitled Session"}
                            </span>
                          </div>
                          <div
                            className="flex items-center gap-1 text-[10px]"
                            style={{ color: "hsl(220 10% 45%)" }}
                          >
                            <Clock className="w-2.5 h-2.5" />
                            <span>{formatRelativeTime(session.updatedAt)}</span>
                          </div>
                        </>
                      )}
                    </div>

                    {/* Menu */}
                    {!isEditing && (
                      <DropdownMenu onOpenChange={(open) => setOpenMenuId(open ? session.id : null)}>
                        <DropdownMenuTrigger asChild>
                          <button
                            className={cn(
                              "w-6 h-6 rounded flex items-center justify-center flex-shrink-0",
                              "transition-all duration-150",
                              (isMenuOpen || isSelected)
                                ? "opacity-100"
                                : "opacity-0 group-hover:opacity-100"
                            )}
                            style={{
                              background: isMenuOpen ? "hsla(220 10% 100% / 0.08)" : "transparent",
                            }}
                            onClick={(e) => e.stopPropagation()}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.background = "hsla(220 10% 100% / 0.08)";
                            }}
                            onMouseLeave={(e) => {
                              if (!isMenuOpen) {
                                e.currentTarget.style.background = "transparent";
                              }
                            }}
                          >
                            <MoreHorizontal className="w-3.5 h-3.5" style={{ color: "hsl(220 10% 50%)" }} />
                          </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent
                          align="end"
                          className="w-40"
                          style={{
                            background: "hsl(220 10% 14%)",
                            border: "1px solid hsla(220 10% 100% / 0.08)",
                            boxShadow: "0 8px 32px hsla(0 0% 0% / 0.4)",
                          }}
                        >
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation();
                              handleStartRename(session);
                            }}
                            className="text-[13px] cursor-pointer gap-2.5 py-2"
                          >
                            <Pencil className="w-3.5 h-3.5" />
                            Rename
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation();
                              onArchiveSession?.(session.id);
                            }}
                            className="text-[13px] cursor-pointer gap-2.5 py-2"
                          >
                            <Archive className="w-3.5 h-3.5" />
                            Archive
                          </DropdownMenuItem>
                          <DropdownMenuSeparator style={{ background: "hsla(220 10% 100% / 0.06)" }} />
                          <DropdownMenuItem
                            onClick={(e) => {
                              e.stopPropagation();
                              onDeleteSession?.(session.id);
                            }}
                            className="text-[13px] cursor-pointer gap-2.5 py-2"
                            style={{ color: "hsl(0 70% 60%)" }}
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                            Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
          )}
        </div>
      </div>
    </div>
  );
}
