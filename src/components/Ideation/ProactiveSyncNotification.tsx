/**
 * ProactiveSyncNotification - Notification banner for plan sync updates
 */

import { AlertCircle, Eye, Undo2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ProactiveSyncNotification } from "@/stores/ideationStore";

interface ProactiveSyncNotificationProps {
  notification: ProactiveSyncNotification;
  onDismiss: () => void;
  onReview: () => void;
  onUndo: () => void;
}

export function ProactiveSyncNotificationBanner({
  notification,
  onDismiss,
  onReview,
  onUndo,
}: ProactiveSyncNotificationProps) {
  const affectedCount = notification.proposalIds.length;

  return (
    <div
      data-testid="proactive-sync-notification"
      className="mb-3 p-3 rounded-lg bg-gradient-to-br from-[#ff6b35]/10 to-[#ff6b35]/5 border border-[#ff6b35]/30"
    >
      <div className="flex items-start gap-2">
        <div className="w-7 h-7 rounded-md bg-[#ff6b35]/20 flex items-center justify-center flex-shrink-0">
          <AlertCircle className="w-3.5 h-3.5 text-[#ff6b35]" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-xs font-medium text-[var(--text-primary)] mb-0.5">Plan updated</p>
          <p className="text-[11px] text-[var(--text-secondary)]">
            {affectedCount} proposal{affectedCount !== 1 ? "s" : ""} may need revision.
          </p>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={onReview}
            className="h-6 px-2 text-[11px] text-[#ff6b35] hover:bg-[#ff6b35]/10"
          >
            <Eye className="w-3 h-3 mr-1" /> Review
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={onUndo}
            className="h-6 px-2 text-[11px] hover:bg-white/[0.06]"
          >
            <Undo2 className="w-3 h-3 mr-1" /> Undo
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={onDismiss}
            className="h-6 w-6 hover:bg-white/[0.06]"
          >
            ×
          </Button>
        </div>
      </div>
    </div>
  );
}
