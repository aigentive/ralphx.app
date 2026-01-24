/**
 * MergeWorkflowDialog - Modal for handling post-completion workflow when a project
 * finishes in worktree mode.
 *
 * Presents options for:
 * - Merge to main (creates merge commit)
 * - Rebase onto main (linear history)
 * - Create Pull Request (review first)
 * - Keep worktree (merge manually later)
 * - Discard changes (delete worktree and branch)
 */

import { useState, useCallback, useEffect } from "react";
import type { Project } from "@/types/project";

// ============================================================================
// Types
// ============================================================================

/**
 * Available merge workflow options
 */
export type MergeOption =
  | "merge"
  | "rebase"
  | "create_pr"
  | "keep_worktree"
  | "discard";

/**
 * Completion data passed to the dialog
 */
export interface CompletionData {
  /** Number of commits made by RalphX */
  commitCount: number;
  /** Branch name created by RalphX */
  branchName: string;
}

/**
 * Result returned when workflow is confirmed
 */
export interface MergeWorkflowResult {
  option: MergeOption;
  project: Project;
}

// ============================================================================
// Props Interface
// ============================================================================

export interface MergeWorkflowDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Callback when the dialog is closed/cancelled */
  onClose: () => void;
  /** Callback when a workflow option is confirmed */
  onConfirm: (result: MergeWorkflowResult) => void;
  /** Project that completed */
  project: Project;
  /** Completion data (commit count, branch name) */
  completionData: CompletionData;
  /** Callback to view diff (opens diff viewer) */
  onViewDiff?: () => void;
  /** Callback to view commits (opens commit history) */
  onViewCommits?: () => void;
  /** Whether workflow is in progress */
  isProcessing?: boolean;
  /** Error message to display */
  error?: string | null;
}

// ============================================================================
// Icons
// ============================================================================

function CloseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M12 4L4 12M4 4l8 8"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

function WarningIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <path
        d="M7 1L13 12H1L7 1z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M7 5v3M7 10v.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

function CheckCircleIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <circle cx="10" cy="10" r="8" stroke="currentColor" strokeWidth="1.5" />
      <path
        d="M6.5 10.5l2 2 5-5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function MergeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <circle cx="4" cy="4" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="4" cy="12" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="12" cy="8" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M4 5.5V10.5M10.5 8H6C6 8 6 4 4 4" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

function RebaseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <circle cx="4" cy="3" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="4" cy="8" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="4" cy="13" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M4 4.5V6.5M4 9.5V11.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M8 3h4M8 8h4M8 13h4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function PullRequestIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <circle cx="4" cy="4" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="4" cy="12" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <circle cx="12" cy="12" r="1.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M4 5.5V10.5M12 5V10.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M12 5L10 3M12 5L14 3" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function WorktreeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <rect x="2" y="2" width="5" height="5" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <rect x="9" y="9" width="5" height="5" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <path d="M7 4.5h2M11.5 7v2" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M2.5 4h11M5.5 4V3a1 1 0 011-1h3a1 1 0 011 1v1M12 4v9a1 1 0 01-1 1H5a1 1 0 01-1-1V4"
        stroke="currentColor"
        strokeWidth="1.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path d="M6.5 7v4M9.5 7v4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
    </svg>
  );
}

function DiffIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <rect x="2" y="2" width="10" height="10" rx="1" stroke="currentColor" strokeWidth="1.2" />
      <path d="M5 6h4M5 8h2" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      <path d="M2 5h10" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

function CommitIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
      <circle cx="7" cy="7" r="2.5" stroke="currentColor" strokeWidth="1.2" />
      <path d="M7 1v3.5M7 9.5V13" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

// ============================================================================
// Option Configuration
// ============================================================================

interface MergeOptionConfig {
  value: MergeOption;
  label: string;
  description: string;
  icon: React.ReactNode;
  destructive?: boolean;
  warning?: string;
}

const MERGE_OPTIONS: MergeOptionConfig[] = [
  {
    value: "merge",
    label: "Merge to main",
    description: "Creates a merge commit preserving branch history",
    icon: <MergeIcon />,
  },
  {
    value: "rebase",
    label: "Rebase onto main",
    description: "Replays commits on top of main for linear history",
    icon: <RebaseIcon />,
  },
  {
    value: "create_pr",
    label: "Create Pull Request",
    description: "Opens GitHub to create a PR for code review",
    icon: <PullRequestIcon />,
  },
  {
    value: "keep_worktree",
    label: "Keep worktree",
    description: "Leave as-is and merge manually later",
    icon: <WorktreeIcon />,
  },
  {
    value: "discard",
    label: "Discard changes",
    description: "Delete the worktree and branch permanently",
    icon: <TrashIcon />,
    destructive: true,
    warning: "This cannot be undone. All commits will be lost.",
  },
];

// ============================================================================
// Sub-components
// ============================================================================

interface RadioOptionProps {
  config: MergeOptionConfig;
  selected: boolean;
  onSelect: () => void;
  disabled?: boolean;
}

function RadioOption({
  config,
  selected,
  onSelect,
  disabled,
}: RadioOptionProps) {
  return (
    <label
      data-testid={`merge-option-${config.value}`}
      data-selected={selected ? "true" : "false"}
      className={`flex gap-3 p-3 rounded-lg transition-colors ${
        disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"
      }`}
      style={{
        backgroundColor: selected ? "var(--bg-elevated)" : "transparent",
        border: `1px solid ${
          selected
            ? config.destructive
              ? "var(--status-error)"
              : "var(--accent-primary)"
            : "var(--border-subtle)"
        }`,
      }}
    >
      <input
        type="radio"
        name="mergeOption"
        value={config.value}
        checked={selected}
        onChange={onSelect}
        disabled={disabled}
        className="sr-only"
      />
      <span
        className="mt-0.5 w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0"
        style={{
          borderColor: selected
            ? config.destructive
              ? "var(--status-error)"
              : "var(--accent-primary)"
            : "var(--border-subtle)",
        }}
      >
        {selected && (
          <span
            className="w-2 h-2 rounded-full"
            style={{
              backgroundColor: config.destructive
                ? "var(--status-error)"
                : "var(--accent-primary)",
            }}
          />
        )}
      </span>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span
            style={{
              color: config.destructive
                ? "var(--status-error)"
                : "var(--text-primary)",
            }}
          >
            {config.icon}
          </span>
          <span
            className="text-sm font-medium"
            style={{
              color: config.destructive
                ? "var(--status-error)"
                : "var(--text-primary)",
            }}
          >
            {config.label}
          </span>
        </div>
        <div className="text-xs mt-0.5" style={{ color: "var(--text-muted)" }}>
          {config.description}
        </div>
        {selected && config.warning && (
          <div
            className="flex items-center gap-1.5 text-xs mt-2"
            style={{ color: "var(--status-warning)" }}
          >
            <WarningIcon />
            <span>{config.warning}</span>
          </div>
        )}
      </div>
    </label>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function MergeWorkflowDialog({
  isOpen,
  onClose,
  onConfirm,
  project,
  completionData,
  onViewDiff,
  onViewCommits,
  isProcessing = false,
  error = null,
}: MergeWorkflowDialogProps) {
  const [selectedOption, setSelectedOption] = useState<MergeOption>("merge");
  const [showDiscardConfirm, setShowDiscardConfirm] = useState(false);

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen) {
      setSelectedOption("merge");
      setShowDiscardConfirm(false);
    }
  }, [isOpen]);

  // Handle confirm
  const handleConfirm = useCallback(() => {
    // If discard is selected and not yet confirmed, show confirmation
    if (selectedOption === "discard" && !showDiscardConfirm) {
      setShowDiscardConfirm(true);
      return;
    }

    onConfirm({
      option: selectedOption,
      project,
    });
  }, [selectedOption, showDiscardConfirm, onConfirm, project]);

  // Handle option selection
  const handleOptionSelect = useCallback((option: MergeOption) => {
    setSelectedOption(option);
    setShowDiscardConfirm(false);
  }, []);

  // Don't render if not open
  if (!isOpen) return null;

  const isDiscardSelected = selectedOption === "discard";
  const buttonLabel = isProcessing
    ? "Processing..."
    : showDiscardConfirm
      ? "Confirm Discard"
      : "Continue";

  return (
    <div
      data-testid="merge-workflow-dialog"
      className="fixed inset-0 z-50 flex items-center justify-center"
    >
      {/* Backdrop */}
      <div
        data-testid="dialog-overlay"
        className="absolute inset-0 transition-opacity"
        style={{ backgroundColor: "rgba(0, 0, 0, 0.5)" }}
        onClick={onClose}
      />

      {/* Modal */}
      <div
        data-testid="dialog-modal"
        className="relative w-full max-w-lg mx-4 rounded-xl shadow-2xl"
        style={{
          backgroundColor: "var(--bg-surface)",
          border: "1px solid var(--border-subtle)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-6 py-4 border-b"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <div className="flex items-center gap-3">
            <span style={{ color: "var(--status-success)" }}>
              <CheckCircleIcon />
            </span>
            <h2
              className="text-lg font-semibold"
              style={{ color: "var(--text-primary)" }}
            >
              Project Complete: {project.name}
            </h2>
          </div>
          <button
            data-testid="dialog-close"
            onClick={onClose}
            disabled={isProcessing}
            className="p-1 rounded transition-colors hover:bg-white/5"
            style={{ color: "var(--text-muted)" }}
          >
            <CloseIcon />
          </button>
        </div>

        {/* Content */}
        <div className="px-6 py-5 space-y-5">
          {/* Summary */}
          <div
            className="text-sm"
            style={{ color: "var(--text-secondary)" }}
          >
            RalphX made{" "}
            <span
              data-testid="commit-count"
              className="font-medium"
              style={{ color: "var(--text-primary)" }}
            >
              {completionData.commitCount} commit
              {completionData.commitCount !== 1 ? "s" : ""}
            </span>{" "}
            on branch:{" "}
            <span
              data-testid="branch-name"
              className="font-mono font-medium"
              style={{ color: "var(--accent-primary)" }}
            >
              {completionData.branchName}
            </span>
          </div>

          {/* Action Buttons */}
          <div className="flex gap-2">
            {onViewDiff && (
              <button
                data-testid="view-diff-button"
                onClick={onViewDiff}
                disabled={isProcessing}
                className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors"
                style={{
                  backgroundColor: "var(--bg-elevated)",
                  color: "var(--text-primary)",
                }}
              >
                <DiffIcon />
                View Diff
              </button>
            )}
            {onViewCommits && (
              <button
                data-testid="view-commits-button"
                onClick={onViewCommits}
                disabled={isProcessing}
                className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors"
                style={{
                  backgroundColor: "var(--bg-elevated)",
                  color: "var(--text-primary)",
                }}
              >
                <CommitIcon />
                View Commits
              </button>
            )}
          </div>

          {/* Divider */}
          <div
            className="h-px"
            style={{ backgroundColor: "var(--border-subtle)" }}
          />

          {/* Question */}
          <div
            className="text-sm font-medium"
            style={{ color: "var(--text-secondary)" }}
          >
            What would you like to do?
          </div>

          {/* Options */}
          <div className="space-y-2">
            {MERGE_OPTIONS.map((config) => (
              <RadioOption
                key={config.value}
                config={config}
                selected={selectedOption === config.value}
                onSelect={() => handleOptionSelect(config.value)}
                disabled={isProcessing}
              />
            ))}
          </div>

          {/* Error Message */}
          {error && (
            <div
              data-testid="dialog-error"
              className="flex items-center gap-2 px-3 py-2 rounded-lg"
              style={{
                backgroundColor: "rgba(239, 68, 68, 0.1)",
                color: "var(--status-error)",
              }}
            >
              <WarningIcon />
              <span className="text-sm">{error}</span>
            </div>
          )}

          {/* Discard Confirmation */}
          {showDiscardConfirm && (
            <div
              data-testid="discard-confirmation"
              className="flex items-center gap-2 px-3 py-2 rounded-lg"
              style={{
                backgroundColor: "rgba(239, 68, 68, 0.1)",
                color: "var(--status-error)",
              }}
            >
              <WarningIcon />
              <span className="text-sm">
                Are you sure? Click "Confirm Discard" to permanently delete the
                worktree and branch.
              </span>
            </div>
          )}
        </div>

        {/* Footer */}
        <div
          className="flex items-center justify-end gap-3 px-6 py-4 border-t"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          <button
            data-testid="cancel-button"
            onClick={onClose}
            disabled={isProcessing}
            className="px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            style={{
              backgroundColor: "var(--bg-elevated)",
              color: "var(--text-primary)",
            }}
          >
            Cancel
          </button>
          <button
            data-testid="confirm-button"
            onClick={handleConfirm}
            disabled={isProcessing}
            className="px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            style={{
              backgroundColor:
                isProcessing
                  ? "var(--bg-hover)"
                  : isDiscardSelected
                    ? "var(--status-error)"
                    : "var(--accent-primary)",
              color: isProcessing ? "var(--text-muted)" : "#fff",
              cursor: isProcessing ? "not-allowed" : "pointer",
            }}
          >
            {buttonLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
