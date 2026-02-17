/**
 * ResumeValidationDialog - Modal for displaying validation failures when resuming a task.
 *
 * Used when a task that was stopped mid-execution (e.g., during Merging) has validation
 * issues that prevent a direct resume. Shows warnings and lets user choose:
 * - Force Resume: Resume anyway, ignoring validation failures
 * - Go to Ready: Reset the task to Ready status for fresh execution
 * - Cancel: Close the dialog and take no action
 */

import { useCallback } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { AlertTriangle, Play, RotateCcw, X, Loader2 } from "lucide-react";

// ============================================================================
// Types
// ============================================================================

export interface ValidationWarning {
  /** Unique identifier for the warning */
  id: string;
  /** Human-readable warning message */
  message: string;
  /** Optional severity level */
  severity?: "warning" | "error";
}

// ============================================================================
// Props Interface
// ============================================================================

export interface ResumeValidationDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Callback when the dialog is closed/cancelled */
  onClose: () => void;
  /** Callback when "Force Resume" is clicked */
  onForceResume: () => void;
  /** Callback when "Go to Ready" is clicked */
  onGoToReady: () => void;
  /** Title of the task being resumed (optional, for display) */
  taskTitle?: string;
  /** Original status the task was stopped from */
  stoppedFromStatus?: string | undefined;
  /** List of validation warnings to display */
  warnings: ValidationWarning[];
  /** Whether an action is in progress */
  isLoading?: boolean;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Format internal status for display (snake_case → Title Case)
 */
function formatStatus(status: string): string {
  return status
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

// ============================================================================
// Main Component
// ============================================================================

export function ResumeValidationDialog({
  isOpen,
  onClose,
  onForceResume,
  onGoToReady,
  taskTitle,
  stoppedFromStatus,
  warnings,
  isLoading = false,
}: ResumeValidationDialogProps) {
  // Handle dialog close
  const handleOpenChange = useCallback(
    (open: boolean) => {
      if (!open && !isLoading) {
        onClose();
      }
    },
    [onClose, isLoading]
  );

  return (
    <Dialog open={isOpen} onOpenChange={handleOpenChange}>
      <DialogContent
        data-testid="resume-validation-dialog"
        className="max-w-md"
      >
        {/* Header */}
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div
              className="p-2 rounded-full"
              style={{
                backgroundColor: "rgba(251, 191, 36, 0.15)",
              }}
            >
              <AlertTriangle
                className="h-5 w-5"
                style={{ color: "var(--status-warning)" }}
              />
            </div>
            <div>
              <DialogTitle data-testid="dialog-title">
                Resume Validation Failed
              </DialogTitle>
              {taskTitle && (
                <DialogDescription className="mt-1">
                  {taskTitle}
                </DialogDescription>
              )}
            </div>
          </div>
        </DialogHeader>

        {/* Original Status */}
        {stoppedFromStatus && (
          <div className="px-6 py-2 flex items-center gap-2">
            <span className="text-sm text-[var(--text-muted)]">
              Stopped from:
            </span>
            <span
              data-testid="stopped-from-status-badge"
              className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-[var(--bg-surface)] text-[var(--text-secondary)] border border-[var(--border-subtle)]"
            >
              {formatStatus(stoppedFromStatus)}
            </span>
          </div>
        )}

        {/* Warnings List */}
        <div className="px-6 py-4">
          <p className="text-sm font-medium text-[var(--text-secondary)] mb-3">
            The following issues were detected:
          </p>
          <ul
            data-testid="validation-warnings-list"
            className="space-y-2"
          >
            {warnings.map((warning) => (
              <li
                key={warning.id}
                data-testid={`warning-${warning.id}`}
                className="flex items-start gap-2 p-2 rounded-md bg-[var(--bg-surface)] border border-[var(--border-subtle)]"
              >
                <AlertTriangle
                  className="h-4 w-4 mt-0.5 flex-shrink-0"
                  style={{
                    color:
                      warning.severity === "error"
                        ? "var(--status-error)"
                        : "var(--status-warning)",
                  }}
                />
                <span className="text-sm text-[var(--text-primary)]">
                  {warning.message}
                </span>
              </li>
            ))}
          </ul>
        </div>

        {/* Footer */}
        <DialogFooter>
          <Button
            data-testid="cancel-button"
            type="button"
            onClick={onClose}
            variant="ghost"
            disabled={isLoading}
            className="bg-[var(--bg-elevated)] text-[var(--text-primary)] hover:bg-[var(--bg-hover)]"
          >
            <X className="h-4 w-4 mr-2" />
            Cancel
          </Button>
          <Button
            data-testid="go-to-ready-button"
            type="button"
            onClick={onGoToReady}
            variant="outline"
            disabled={isLoading}
            className="gap-2"
          >
            {isLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <RotateCcw className="h-4 w-4" />
            )}
            Go to Ready
          </Button>
          <Button
            data-testid="force-resume-button"
            type="button"
            onClick={onForceResume}
            disabled={isLoading}
            className="gap-2"
            style={{
              backgroundColor: "var(--accent-primary)",
              color: "white",
            }}
          >
            {isLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Play className="h-4 w-4" />
            )}
            Force Resume
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
