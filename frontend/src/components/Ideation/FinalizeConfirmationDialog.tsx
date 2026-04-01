/**
 * FinalizeConfirmationDialog - Custom modal for agent-initiated plan finalization gate.
 *
 * Shows when the backend emits ideation:finalize_pending_confirmation.
 * Mounted at App root level so it appears regardless of the active view.
 * Manages a queue of pending confirmations: first item shown, rest wait.
 *
 * Design: Custom modal (NOT AlertDialog) — 3-action layout needs more than binary confirm/cancel.
 */

import { useCallback, useState } from "react";
import { Check, X, Eye, Loader2 } from "lucide-react";
import { useUiStore } from "@/stores/uiStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { useAcceptFinalize, useRejectFinalize } from "@/hooks/useAcceptFinalize";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";

// ============================================================================
// Inner dialog — rendered only when there's an active session in queue
// ============================================================================

interface ActiveDialogProps {
  sessionId: string;
  onViewPlan: () => void;
}

function ActiveDialog({ sessionId, onViewPlan }: ActiveDialogProps) {
  const dequeue = useUiStore((s) => s.dequeueConfirmation);
  const addAutoAcceptSession = useUiStore((s) => s.addAutoAcceptSession);
  const sessions = useIdeationStore((s) => s.sessions);
  const sessionTitle = sessions[sessionId]?.title ?? null;

  const [autoAcceptSession, setAutoAcceptSession] = useState(false);

  const accept = useAcceptFinalize(sessionId);
  const reject = useRejectFinalize(sessionId);

  const isLoading = accept.isPending || reject.isPending;

  const handleAccept = useCallback(async () => {
    try {
      await accept.mutateAsync();
      if (autoAcceptSession) {
        addAutoAcceptSession(sessionId);
      }
      dequeue();
    } catch {
      // Error toast handled in useAcceptFinalize onError; keep dialog open for retry
    }
  }, [accept, autoAcceptSession, addAutoAcceptSession, sessionId, dequeue]);

  const handleReject = useCallback(async () => {
    try {
      await reject.mutateAsync();
      dequeue();
    } catch {
      // Error toast handled in useRejectFinalize onError; keep dialog open for retry
    }
  }, [reject, dequeue]);

  const handleViewPlan = useCallback(() => {
    dequeue();
    onViewPlan();
  }, [dequeue, onViewPlan]);

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-[200]"
        style={{ background: "hsla(220 10% 4% / 0.7)", backdropFilter: "blur(4px)" }}
      />

      {/* Dialog */}
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="finalize-dialog-title"
        className="fixed z-[201] left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-md rounded-2xl overflow-hidden shadow-2xl"
        style={{
          background: "hsl(220 10% 10%)",
          border: "1px solid hsla(220 10% 100% / 0.1)",
          boxShadow: "0 0 60px hsla(14 100% 60% / 0.06), 0 24px 48px hsla(220 10% 0% / 0.6)",
        }}
      >
        {/* Top accent */}
        <div
          className="h-[2px] w-full"
          style={{
            background:
              "linear-gradient(90deg, hsla(14 100% 60% / 0.8) 0%, hsla(14 100% 60% / 0.1) 100%)",
          }}
        />

        <div className="px-6 py-5">
          {/* Header */}
          <div className="mb-4">
            <h2
              id="finalize-dialog-title"
              className="text-[15px] font-semibold mb-1"
              style={{ color: "hsl(220 10% 92%)" }}
            >
              Plan Ready for Acceptance
            </h2>
            <p className="text-[13px]" style={{ color: "hsl(220 10% 50%)" }}>
              The agent has finalized proposals and is waiting for your confirmation.
            </p>
          </div>

          {/* Session title */}
          {sessionTitle && (
            <div
              className="mb-5 px-3 py-2.5 rounded-lg text-[13px] font-medium truncate"
              style={{
                background: "hsla(220 10% 100% / 0.05)",
                border: "1px solid hsla(220 10% 100% / 0.08)",
                color: "hsl(220 10% 80%)",
              }}
            >
              {sessionTitle}
            </div>
          )}

          {/* Per-session auto-accept checkbox */}
          <div className="flex items-center gap-2 mb-5">
            <Checkbox
              id="auto-accept-session"
              checked={autoAcceptSession}
              onCheckedChange={(checked) => { setAutoAcceptSession(checked === true); }}
              disabled={isLoading}
              className="data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
            />
            <Label
              htmlFor="auto-accept-session"
              className="text-[12px] cursor-pointer select-none"
              style={{ color: "hsl(220 10% 60%)" }}
            >
              Auto-accept plans for this session
            </Label>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-2 justify-end">
            {/* View Plan */}
            <button
              onClick={handleViewPlan}
              disabled={isLoading}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[12px] font-medium transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed"
              style={{
                color: "hsl(220 10% 60%)",
                border: "1px solid hsla(220 10% 100% / 0.1)",
                background: "transparent",
              }}
              onMouseEnter={(e) => {
                if (!isLoading) {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                  e.currentTarget.style.color = "hsl(220 10% 80%)";
                }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
                e.currentTarget.style.color = "hsl(220 10% 60%)";
              }}
            >
              <Eye className="w-3.5 h-3.5" />
              View Plan
            </button>

            {/* Reject */}
            <button
              onClick={handleReject}
              disabled={isLoading}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[12px] font-medium transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed"
              style={{
                color: "hsl(0 70% 65%)",
                border: "1px solid hsla(0 70% 60% / 0.2)",
                background: "hsla(0 70% 60% / 0.06)",
              }}
              onMouseEnter={(e) => {
                if (!isLoading) {
                  e.currentTarget.style.background = "hsla(0 70% 60% / 0.12)";
                }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "hsla(0 70% 60% / 0.06)";
              }}
            >
              {reject.isPending ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <X className="w-3.5 h-3.5" />
              )}
              Reject
            </button>

            {/* Accept */}
            <button
              onClick={handleAccept}
              disabled={isLoading}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[12px] font-semibold transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed"
              style={{
                color: "white",
                background: accept.isPending
                  ? "hsl(14 100% 50%)"
                  : "hsl(14 100% 60%)",
                boxShadow: "0 1px 4px hsla(14 100% 40% / 0.3)",
              }}
              onMouseEnter={(e) => {
                if (!isLoading) {
                  e.currentTarget.style.background = "hsl(14 100% 55%)";
                }
              }}
              onMouseLeave={(e) => {
                if (!accept.isPending) {
                  e.currentTarget.style.background = "hsl(14 100% 60%)";
                }
              }}
            >
              {accept.isPending ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <Check className="w-3.5 h-3.5" />
              )}
              Accept
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

// ============================================================================
// Public component — reads queue from uiStore
// ============================================================================

export function FinalizeConfirmationDialog() {
  const queue = useUiStore((s) => s.pendingConfirmationQueue);
  const setCurrentView = useUiStore((s) => s.setCurrentView);
  const setActiveSession = useIdeationStore((s) => s.setActiveSession);

  const activeSessionId = queue[0];

  const handleViewPlan = useCallback(() => {
    if (!activeSessionId) return;
    setActiveSession(activeSessionId);
    setCurrentView("ideation");
  }, [activeSessionId, setActiveSession, setCurrentView]);

  if (!activeSessionId) return null;

  return (
    <ActiveDialog
      key={activeSessionId}
      sessionId={activeSessionId}
      onViewPlan={handleViewPlan}
    />
  );
}
