/**
 * BlockReasonDialog - Modal for capturing an optional reason when blocking a task.
 *
 * Used when the user wants to block a task and optionally provide a reason
 * explaining why the task is blocked.
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
import { Ban } from "lucide-react";

// ============================================================================
// Props Interface
// ============================================================================

export interface BlockReasonDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Callback when the dialog is closed/cancelled */
  onClose: () => void;
  /** Callback when block is confirmed with optional reason */
  onConfirm: (reason?: string) => void;
  /** Title of the task being blocked (optional, for display) */
  taskTitle?: string;
}

// ============================================================================
// Main Component
// ============================================================================

export function BlockReasonDialog({
  isOpen,
  onClose,
  onConfirm,
  taskTitle,
}: BlockReasonDialogProps) {
  const [reason, setReason] = useState("");

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen) {
      setReason("");
    }
  }, [isOpen]);

  // Handle confirm
  const handleConfirm = useCallback(() => {
    const trimmedReason = reason.trim();
    onConfirm(trimmedReason || undefined);
  }, [reason, onConfirm]);

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
        data-testid="block-reason-dialog"
        className="max-w-md"
      >
        {/* Header */}
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div
              className="p-2 rounded-full"
              style={{
                backgroundColor: "rgba(245, 158, 11, 0.15)",
              }}
            >
              <Ban
                className="h-5 w-5"
                style={{ color: "var(--status-warning)" }}
              />
            </div>
            <div>
              <DialogTitle data-testid="dialog-title">Block Task</DialogTitle>
              {taskTitle && (
                <DialogDescription className="mt-1">
                  {taskTitle}
                </DialogDescription>
              )}
            </div>
          </div>
        </DialogHeader>

        {/* Content */}
        <div className="px-6 py-4">
          <label
            htmlFor="block-reason"
            className="text-sm font-medium text-[var(--text-secondary)] block mb-2"
          >
            Reason (optional)
          </label>
          <Textarea
            id="block-reason"
            data-testid="block-reason-input"
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Why is this task blocked?"
            className="min-h-[100px] bg-[var(--bg-surface)] border-[var(--border-subtle)] text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:ring-[var(--accent-primary)] focus:border-[var(--accent-primary)]"
            autoFocus
          />
          <p className="text-xs text-[var(--text-muted)] mt-2">
            Press ⌘+Enter to confirm
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
            data-testid="confirm-button"
            type="button"
            onClick={handleConfirm}
            className="gap-2 bg-[var(--status-warning)] text-white hover:bg-[var(--status-warning)]/90"
          >
            <Ban className="h-4 w-4" />
            Block Task
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
