/**
 * StopReasonDialog - Modal for capturing an optional reason when stopping a task.
 *
 * Used when the user wants to stop a task and optionally provide a reason
 * explaining why the task was stopped.
 */

import { useState, useCallback, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Square, SkipForward } from "lucide-react";
import type { InternalStatus } from "@/types/status";

// ============================================================================
// Props Interface
// ============================================================================

export interface StopReasonDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Callback when the dialog is closed/cancelled */
  onClose: () => void;
  /** Callback when stop is confirmed with optional reason */
  onConfirm: (reason?: string) => void;
  /** Callback when skip is clicked (stop without reason) */
  onSkip: () => void;
  /** Title of the task being stopped (optional, for display) */
  taskTitle?: string;
  /** Current status of the task being stopped */
  taskStatus?: InternalStatus;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Format internal status for display (snake_case → Title Case)
 */
function formatStatus(status: InternalStatus): string {
  return status
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

// ============================================================================
// Main Component
// ============================================================================

export function StopReasonDialog({
  isOpen,
  onClose,
  onConfirm,
  onSkip,
  taskTitle,
  taskStatus,
}: StopReasonDialogProps) {
  const [reason, setReason] = useState("");

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen) {
      setReason("");
    }
  }, [isOpen]);

  // Handle confirm with reason
  const handleConfirm = useCallback(() => {
    const trimmedReason = reason.trim();
    onConfirm(trimmedReason || undefined);
  }, [reason, onConfirm]);

  // Handle skip (stop without reason)
  const handleSkip = useCallback(() => {
    onSkip();
  }, [onSkip]);

  // Handle dialog close
  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (!open) {
        onClose();
      }
    },
    [onClose]
  );

  // Handle keyboard submit
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && e.metaKey) {
        e.preventDefault();
        handleConfirm();
      }
    },
    [handleConfirm]
  );

  return (
    <Dialog open={isOpen} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="stop-reason-dialog"
        className="max-w-md"
      >
        {/* Header */}
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div
              className="p-2 rounded-full"
              style={{
                backgroundColor: "var(--status-error-muted)",
              }}
            >
              <Square
                className="h-5 w-5"
                style={{ color: "var(--status-error)" }}
              />
            </div>
            <div>
              <DialogTitle data-testid="dialog-title">Stop Task</DialogTitle>
              {taskTitle && (
                <DialogDescription className="mt-1">
                  {taskTitle}
                </DialogDescription>
              )}
            </div>
          </div>
        </DialogHeader>

        {/* Task Status */}
        {taskStatus && (
          <div className="px-6 py-2 flex items-center gap-2">
            <span className="text-sm text-[var(--text-muted)]">Current status:</span>
            <span
              data-testid="task-status-badge"
              className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-[var(--bg-surface)] text-[var(--text-secondary)] border border-[var(--border-subtle)]"
            >
              {formatStatus(taskStatus)}
            </span>
          </div>
        )}

        {/* Content */}
        <div className="px-6 py-4">
          <label
            htmlFor="stop-reason"
            className="text-sm font-medium text-[var(--text-secondary)] block mb-2"
          >
            Reason (optional)
          </label>
          <Textarea
            id="stop-reason"
            data-testid="stop-reason-input"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Why are you stopping this task?"
            className="min-h-[100px] bg-[var(--bg-surface)] border-[var(--border-subtle)] text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:ring-[var(--accent-primary)] focus:border-[var(--accent-primary)]"
            autoFocus
          />
          <p className="text-xs text-[var(--text-muted)] mt-2">
            Press ⌘+Enter to confirm with reason
          </p>
        </div>

        {/* Footer */}
        <DialogFooter>
          <Button
            data-testid="cancel-button"
            type="button"
            onClick={onClose}
            variant="ghost"
            className="bg-[var(--bg-elevated)] text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
          >
            Cancel
          </Button>
          <Button
            data-testid="skip-button"
            type="button"
            onClick={handleSkip}
            variant="outline"
            className="gap-2"
          >
            <SkipForward className="h-4 w-4" />
            Skip
          </Button>
          <Button
            data-testid="confirm-button"
            type="button"
            onClick={handleConfirm}
            className="gap-2 bg-[var(--status-error)] text-white hover:bg-[var(--status-error)]/90"
          >
            <Square className="h-4 w-4" />
            Stop Task
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
