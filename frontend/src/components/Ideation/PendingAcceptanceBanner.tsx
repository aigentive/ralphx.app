/**
 * PendingAcceptanceBanner - Inline banner shown after "View Plan" closes the dialog.
 *
 * Follows AcceptedSessionBanner.tsx visual pattern.
 * Shown in PlanningView when session.acceptanceStatus === "pending".
 */

import { AlertTriangle, Check, X, Loader2 } from "lucide-react";
import { useAcceptFinalize, useRejectFinalize } from "@/hooks/useAcceptFinalize";
import { useUiStore } from "@/stores/uiStore";

interface PendingAcceptanceBannerProps {
  sessionId: string;
}

export function PendingAcceptanceBanner({ sessionId }: PendingAcceptanceBannerProps) {
  const removeFromQueue = useUiStore((s) => s.removeFromConfirmationQueue);
  const accept = useAcceptFinalize(sessionId);
  const reject = useRejectFinalize(sessionId);

  const isLoading = accept.isPending || reject.isPending;

  async function handleAccept() {
    try {
      await accept.mutateAsync();
      removeFromQueue(sessionId);
    } catch {
      // Error toast handled in useAcceptFinalize onError
    }
  }

  async function handleReject() {
    try {
      await reject.mutateAsync();
      removeFromQueue(sessionId);
    } catch {
      // Error toast handled in useRejectFinalize onError
    }
  }

  return (
    <div
      data-testid="pending-acceptance-banner"
      className="mb-4 rounded-xl overflow-hidden"
      style={{
        background: "hsla(220 10% 14% / 0.8)",
        border: "1px solid hsla(40 90% 55% / 0.2)",
        boxShadow: "0 0 24px hsla(40 90% 55% / 0.04)",
      }}
    >
      {/* Top accent line */}
      <div
        className="h-[2px] w-full"
        style={{
          background:
            "linear-gradient(90deg, hsla(40 90% 55% / 0.6) 0%, hsla(40 90% 55% / 0.1) 100%)",
        }}
      />

      <div className="px-4 py-3.5 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div
            className="w-5 h-5 rounded-full flex items-center justify-center flex-shrink-0"
            style={{ background: "hsla(40 90% 55% / 0.15)" }}
          >
            <AlertTriangle className="w-3 h-3" style={{ color: "hsl(40 90% 55%)" }} />
          </div>
          <span className="text-[13px] font-medium" style={{ color: "hsl(220 10% 90%)" }}>
            Plan pending your confirmation
          </span>
        </div>

        <div className="flex items-center gap-2">
          {/* Reject */}
          <button
            onClick={handleReject}
            disabled={isLoading}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-[11px] font-medium transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed"
            style={{
              color: "hsl(0 70% 65%)",
              border: "1px solid hsla(0 70% 60% / 0.2)",
              background: "hsla(0 70% 60% / 0.06)",
            }}
            onMouseEnter={(e) => {
              if (!isLoading) e.currentTarget.style.background = "hsla(0 70% 60% / 0.12)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "hsla(0 70% 60% / 0.06)";
            }}
          >
            {reject.isPending ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <X className="w-3 h-3" />
            )}
            Reject
          </button>

          {/* Accept */}
          <button
            onClick={handleAccept}
            disabled={isLoading}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-[11px] font-semibold transition-all duration-150 disabled:opacity-40 disabled:cursor-not-allowed"
            style={{
              color: "white",
              background: accept.isPending ? "hsl(14 100% 50%)" : "hsl(14 100% 60%)",
              boxShadow: "0 1px 4px hsla(14 100% 40% / 0.3)",
            }}
            onMouseEnter={(e) => {
              if (!isLoading) e.currentTarget.style.background = "hsl(14 100% 55%)";
            }}
            onMouseLeave={(e) => {
              if (!accept.isPending) e.currentTarget.style.background = "hsl(14 100% 60%)";
            }}
          >
            {accept.isPending ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <Check className="w-3 h-3" />
            )}
            Accept
          </button>
        </div>
      </div>
    </div>
  );
}
