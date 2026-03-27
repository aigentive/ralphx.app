/**
 * ChildSessionNotification - Shows verification started notification
 *
 * Listens for verification child session creation via store state and displays
 * a "Verification started" banner with a "View" button that switches to the
 * Verification tab. General follow-up navigation is handled inline by ChildSessionWidget.
 */

import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { ShieldCheck, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useIdeationStore } from "@/stores/ideationStore";

interface ChildSessionNotificationProps {
  /** Current session ID to filter notifications */
  sessionId: string;
}

/**
 * Shows a notification when a verification child session is created for the current session.
 * Drives tab switching to the Verification panel via store actions.
 */
export function ChildSessionNotification({
  sessionId,
}: ChildSessionNotificationProps) {
  // Read verification notification from store (set by useIdeationEvents on purpose=verification)
  const verificationChildId = useIdeationStore((s) => s.verificationNotifications[sessionId]);
  const setActiveIdeationTab = useIdeationStore((s) => s.setActiveIdeationTab);
  const setActiveVerificationChildId = useIdeationStore((s) => s.setActiveVerificationChildId);
  const clearVerificationNotification = useIdeationStore((s) => s.clearVerificationNotification);

  // Reactive selector — re-evaluates when session verification state changes in the store
  const sessionVerificationState = useIdeationStore(
    useShallow((s) => {
      const session = s.sessions[sessionId];
      if (!session) return null;
      return {
        verificationStatus: session.verificationStatus,
        verificationInProgress: session.verificationInProgress,
      };
    }),
  );

  // Reconciliation effect — clears stale notifications when session reaches terminal state.
  // Handles agent crash or missing terminal event where the normal clear path never fires.
  useEffect(() => {
    if (!verificationChildId || !sessionVerificationState) return;

    const { verificationStatus, verificationInProgress } = sessionVerificationState;

    // Terminal: not in progress AND not currently reviewing or unverified
    const isTerminal =
      !verificationInProgress &&
      verificationStatus !== "unverified" &&
      verificationStatus !== "reviewing";

    if (isTerminal) {
      clearVerificationNotification(sessionId);
    }
  }, [sessionId, verificationChildId, sessionVerificationState, clearVerificationNotification]);

  if (!verificationChildId) {
    return null;
  }

  const handleViewVerification = () => {
    setActiveIdeationTab(sessionId, 'verification');
    setActiveVerificationChildId(sessionId, verificationChildId ?? null);
  };

  return (
    <div
      data-testid="verification-started-notification"
      className="mx-4 mb-2 p-3 rounded-lg animate-[slide-in-bottom_300ms_ease-out]"
      style={{
        background: "hsla(220 10% 14% / 0.8)",
        border: "1px solid hsla(220 20% 100% / 0.08)",
        backdropFilter: "blur(8px)",
      }}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 flex-1 min-w-0">
          <ShieldCheck
            className="w-3.5 h-3.5 shrink-0"
            style={{ color: "hsl(220 10% 55%)" }}
          />
          <p
            className="text-sm font-medium"
            style={{ color: "hsl(220 10% 90%)" }}
          >
            Verification started
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Button
            size="sm"
            onClick={handleViewVerification}
            data-testid="view-verification-button"
            className="gap-2 h-7 text-xs"
            style={{
              background: "hsla(14 100% 60% / 0.12)",
              border: "1px solid hsla(14 100% 60% / 0.2)",
              color: "hsl(14 100% 60%)",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "hsla(14 100% 60% / 0.18)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "hsla(14 100% 60% / 0.12)";
            }}
          >
            View
          </Button>
          <button
            onClick={() => clearVerificationNotification(sessionId)}
            data-testid="dismiss-verification-button"
            className="w-5 h-5 flex items-center justify-center rounded opacity-50 hover:opacity-100 transition-opacity"
            style={{ color: "hsl(220 10% 60%)" }}
            aria-label="Dismiss verification notification"
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      </div>
    </div>
  );
}
