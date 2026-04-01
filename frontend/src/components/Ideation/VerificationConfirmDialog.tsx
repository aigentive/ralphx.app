/**
 * VerificationConfirmDialog - Custom modal for agent-initiated plan verification gate.
 *
 * Shows when the backend enqueues a session ID into pendingVerificationQueue.
 * Mounted at App root level so it appears regardless of the active view.
 * Manages a queue of pending verifications: first item shown, rest wait.
 *
 * Design: Custom modal (NOT AlertDialog) — specialist checkboxes + 3-action layout.
 */

import { useCallback, useState, useEffect } from "react";
import { Check, X, Eye, Loader2, AlertCircle } from "lucide-react";
import { useUiStore } from "@/stores/uiStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { verificationApi } from "@/api/verification";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import type { SpecialistEntry } from "@/types/verification-config";

// ============================================================================
// Inner dialog — rendered only when there's an active session in queue
// ============================================================================

interface ActiveDialogProps {
  sessionId: string;
  onViewPlan: () => void;
}

function ActiveDialog({ sessionId, onViewPlan }: ActiveDialogProps) {
  const dequeue = useUiStore((s) => s.dequeueVerification);
  const addAutoAcceptVerificationSession = useUiStore((s) => s.addAutoAcceptVerificationSession);
  const sessions = useIdeationStore((s) => s.sessions);
  const sessionTitle = sessions[sessionId]?.title ?? null;

  const [autoAcceptSession, setAutoAcceptSession] = useState(false);
  const [specialists, setSpecialists] = useState<SpecialistEntry[]>([]);
  const [specialistsError, setSpecialistsError] = useState(false);
  const [disabledSpecialists, setDisabledSpecialists] = useState<Set<string>>(new Set());

  const [isAccepting, setIsAccepting] = useState(false);
  const [isRejecting, setIsRejecting] = useState(false);

  const isLoading = isAccepting || isRejecting;

  // Fetch specialist list on mount
  useEffect(() => {
    let cancelled = false;
    verificationApi
      .getSpecialists()
      .then((res) => {
        if (cancelled) return;
        setSpecialists(res.specialists);
        // Pre-disable specialists where enabled_by_default is false
        const initialDisabled = new Set(
          res.specialists
            .filter((s) => !s.enabled_by_default)
            .map((s) => s.name)
        );
        setDisabledSpecialists(initialDisabled);
      })
      .catch(() => {
        if (cancelled) return;
        setSpecialistsError(true);
        // On failure: remain usable with no specialists pre-checked as disabled
      });
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  const handleToggleSpecialist = useCallback(
    (name: string, checked: boolean) => {
      setDisabledSpecialists((prev) => {
        const next = new Set(prev);
        if (checked) {
          // checked = enabled → remove from disabled set
          next.delete(name);
        } else {
          // unchecked = disabled → add to disabled set
          next.add(name);
        }
        return next;
      });
    },
    []
  );

  const handleAccept = useCallback(async () => {
    setIsAccepting(true);
    try {
      await verificationApi.confirm(sessionId, Array.from(disabledSpecialists));
      if (autoAcceptSession) {
        addAutoAcceptVerificationSession(sessionId);
      }
      dequeue();
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Failed to confirm verification";
      toast.error(message);
      // Keep dialog open for retry
    } finally {
      setIsAccepting(false);
    }
  }, [sessionId, disabledSpecialists, autoAcceptSession, addAutoAcceptVerificationSession, dequeue]);

  const handleReject = useCallback(async () => {
    setIsRejecting(true);
    try {
      await verificationApi.dismiss(sessionId);
    } catch {
      // Ignore dismiss errors — always dequeue
    } finally {
      setIsRejecting(false);
      dequeue();
    }
  }, [sessionId, dequeue]);

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
        aria-labelledby="verification-dialog-title"
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
              id="verification-dialog-title"
              className="text-[15px] font-semibold mb-1"
              style={{ color: "hsl(220 10% 92%)" }}
            >
              Plan Ready for Verification
            </h2>
            <p className="text-[13px]" style={{ color: "hsl(220 10% 50%)" }}>
              The agent is waiting for your confirmation to start adversarial verification.
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

          {/* Specialist selection */}
          {specialistsError ? (
            <div
              className="mb-5 flex items-center gap-2 px-3 py-2.5 rounded-lg text-[12px]"
              style={{
                background: "hsla(40 80% 50% / 0.08)",
                border: "1px solid hsla(40 80% 50% / 0.2)",
                color: "hsl(40 80% 65%)",
              }}
            >
              <AlertCircle className="w-3.5 h-3.5 shrink-0" />
              Could not load specialist list — all specialists will run.
            </div>
          ) : specialists.length > 0 ? (
            <div className="mb-5">
              <p
                className="text-[11px] font-medium mb-2 uppercase tracking-wider"
                style={{ color: "hsl(220 10% 40%)" }}
              >
                Specialists
              </p>
              <div className="flex flex-col gap-2">
                {specialists.map((specialist) => {
                  const isEnabled = !disabledSpecialists.has(specialist.name);
                  return (
                    <div key={specialist.name} className="flex items-start gap-2.5">
                      <Checkbox
                        id={`specialist-${specialist.name}`}
                        checked={isEnabled}
                        onCheckedChange={(checked) => {
                          handleToggleSpecialist(specialist.name, checked === true);
                        }}
                        disabled={isLoading}
                        className="mt-0.5 data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
                      />
                      <div className="flex flex-col min-w-0">
                        <Label
                          htmlFor={`specialist-${specialist.name}`}
                          className="text-[12px] font-medium cursor-pointer select-none"
                          style={{ color: "hsl(220 10% 70%)" }}
                        >
                          {specialist.display_name}
                        </Label>
                        {specialist.description && (
                          <span
                            className="text-[11px] leading-tight mt-0.5"
                            style={{ color: "hsl(220 10% 40%)" }}
                          >
                            {specialist.description}
                          </span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ) : null}

          {/* Per-session auto-accept checkbox */}
          <div className="flex items-center gap-2 mb-5">
            <Checkbox
              id="auto-accept-verification-session"
              checked={autoAcceptSession}
              onCheckedChange={(checked) => {
                setAutoAcceptSession(checked === true);
              }}
              disabled={isLoading}
              className="data-[state=checked]:bg-[var(--accent-primary)] data-[state=checked]:border-[var(--accent-primary)]"
            />
            <Label
              htmlFor="auto-accept-verification-session"
              className="text-[12px] cursor-pointer select-none"
              style={{ color: "hsl(220 10% 60%)" }}
            >
              Auto-accept verifications for this session
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
              {isRejecting ? (
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
                background: isAccepting ? "hsl(14 100% 50%)" : "hsl(14 100% 60%)",
                boxShadow: "0 1px 4px hsla(14 100% 40% / 0.3)",
              }}
              onMouseEnter={(e) => {
                if (!isLoading) {
                  e.currentTarget.style.background = "hsl(14 100% 55%)";
                }
              }}
              onMouseLeave={(e) => {
                if (!isAccepting) {
                  e.currentTarget.style.background = "hsl(14 100% 60%)";
                }
              }}
            >
              {isAccepting ? (
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

export function VerificationConfirmDialog() {
  const queue = useUiStore((s) => s.pendingVerificationQueue);
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
