/**
 * TaskRerunDialog - Modal for handling re-run workflow when dragging a Done
 * task back to Planned.
 *
 * Presents options for:
 * - Keep changes (recommended) - AI sees current state
 * - Revert commit - Undo the original work
 * - Create new task - Keep original completed, spawn new
 */

import { useState, useCallback, useEffect } from "react";
import type { Task } from "@/types/task";

// ============================================================================
// Types
// ============================================================================

/**
 * Available re-run options
 */
export type RerunOption = "keep_changes" | "revert_commit" | "create_new";

/**
 * Commit information for the completed task
 */
export interface CommitInfo {
  /** Short SHA of the commit */
  sha: string;
  /** Commit message */
  message: string;
  /** Whether there are commits that depend on this one */
  hasDependentCommits: boolean;
}

/**
 * Result returned when re-run is confirmed
 */
export interface TaskRerunResult {
  option: RerunOption;
  task: Task;
}

// ============================================================================
// Props Interface
// ============================================================================

export interface TaskRerunDialogProps {
  /** Whether the dialog is open */
  isOpen: boolean;
  /** Callback when the dialog is closed/cancelled */
  onClose: () => void;
  /** Callback when a re-run option is confirmed */
  onConfirm: (result: TaskRerunResult) => void;
  /** Task being re-run */
  task: Task;
  /** Commit information associated with the task */
  commitInfo: CommitInfo;
  /** Whether re-run is in progress */
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
      <path
        d="M7 5v3M7 10v.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

function RefreshIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
      <path
        d="M14.5 5.5A6.5 6.5 0 1017 10"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <path
        d="M14 2v4h4"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M3 8l4 4 6-8"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function RevertIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M3 6l3-3M3 6l3 3"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M3 6h8a3 3 0 010 6H8"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
      <path
        d="M8 3v10M3 8h10"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

// ============================================================================
// Option Configuration
// ============================================================================

interface RerunOptionConfig {
  value: RerunOption;
  label: string;
  description: string;
  icon: React.ReactNode;
  recommended?: boolean;
  warning?: boolean;
}

const RERUN_OPTIONS: RerunOptionConfig[] = [
  {
    value: "keep_changes",
    label: "Keep changes, run task again",
    description:
      "AI will see current code state and make additional changes if needed",
    icon: <CheckIcon />,
    recommended: true,
  },
  {
    value: "revert_commit",
    label: "Revert commit, then run task",
    description: "Undo the previous work before re-executing",
    icon: <RevertIcon />,
    warning: true,
  },
  {
    value: "create_new",
    label: "Create new task instead",
    description: "Original task stays completed, new task created",
    icon: <PlusIcon />,
  },
];

// ============================================================================
// Sub-components
// ============================================================================

interface RadioOptionProps {
  config: RerunOptionConfig;
  selected: boolean;
  onSelect: () => void;
  disabled?: boolean;
  showWarning?: boolean;
}

function RadioOption({
  config,
  selected,
  onSelect,
  disabled,
  showWarning,
}: RadioOptionProps) {
  const isWarningOption = config.warning && selected && showWarning;

  return (
    <label
      data-testid={`rerun-option-${config.value}`}
      data-selected={selected ? "true" : "false"}
      className={`flex gap-3 p-3 rounded-lg transition-colors ${
        disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"
      }`}
      style={{
        backgroundColor: selected ? "var(--bg-elevated)" : "transparent",
        border: `1px solid ${
          selected
            ? isWarningOption
              ? "var(--status-warning)"
              : "var(--accent-primary)"
            : "var(--border-subtle)"
        }`,
      }}
    >
      <input
        type="radio"
        name="rerunOption"
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
            ? isWarningOption
              ? "var(--status-warning)"
              : "var(--accent-primary)"
            : "var(--border-subtle)",
        }}
      >
        {selected && (
          <span
            className="w-2 h-2 rounded-full"
            style={{
              backgroundColor: isWarningOption
                ? "var(--status-warning)"
                : "var(--accent-primary)",
            }}
          />
        )}
      </span>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span
            style={{
              color: isWarningOption
                ? "var(--status-warning)"
                : "var(--text-primary)",
            }}
          >
            {config.icon}
          </span>
          <span
            className="text-sm font-medium"
            style={{
              color: isWarningOption
                ? "var(--status-warning)"
                : "var(--text-primary)",
            }}
          >
            {config.label}
          </span>
          {config.recommended && (
            <span
              className="px-1.5 py-0.5 text-xs rounded"
              style={{
                backgroundColor: "rgba(255, 107, 53, 0.15)",
                color: "var(--accent-primary)",
              }}
            >
              Recommended
            </span>
          )}
        </div>
        <div className="text-xs mt-0.5" style={{ color: "var(--text-muted)" }}>
          {config.description}
        </div>
      </div>
    </label>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function TaskRerunDialog({
  isOpen,
  onClose,
  onConfirm,
  task,
  commitInfo,
  isProcessing = false,
  error = null,
}: TaskRerunDialogProps) {
  const [selectedOption, setSelectedOption] =
    useState<RerunOption>("keep_changes");

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen) {
      setSelectedOption("keep_changes");
    }
  }, [isOpen]);

  // Handle confirm
  const handleConfirm = useCallback(() => {
    onConfirm({
      option: selectedOption,
      task,
    });
  }, [selectedOption, onConfirm, task]);

  // Handle option selection
  const handleOptionSelect = useCallback((option: RerunOption) => {
    setSelectedOption(option);
  }, []);

  // Don't render if not open
  if (!isOpen) return null;

  const isRevertSelected = selectedOption === "revert_commit";
  const showDependentWarning = isRevertSelected && commitInfo.hasDependentCommits;

  return (
    <div
      data-testid="task-rerun-dialog"
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
            <span style={{ color: "var(--accent-primary)" }}>
              <RefreshIcon />
            </span>
            <h2
              data-testid="dialog-title"
              className="text-lg font-semibold"
              style={{ color: "var(--text-primary)" }}
            >
              Re-run Task
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
          {/* Task Info */}
          <div>
            <div
              data-testid="task-title"
              className="text-base font-medium"
              style={{ color: "var(--text-primary)" }}
            >
              "{task.title}"
            </div>
          </div>

          {/* Commit Info */}
          <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
            <span>This task was completed with commit: </span>
            <span
              data-testid="commit-sha"
              className="font-mono font-medium"
              style={{ color: "var(--accent-primary)" }}
            >
              {commitInfo.sha}
            </span>
            <div
              data-testid="commit-message"
              className="mt-1 italic"
              style={{ color: "var(--text-muted)" }}
            >
              "{commitInfo.message}"
            </div>
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
            How should we handle the previous work?
          </div>

          {/* Options */}
          <div className="space-y-2">
            {RERUN_OPTIONS.map((config) => (
              <RadioOption
                key={config.value}
                config={config}
                selected={selectedOption === config.value}
                onSelect={() => handleOptionSelect(config.value)}
                disabled={isProcessing}
                showWarning={commitInfo.hasDependentCommits}
              />
            ))}
          </div>

          {/* Dependent Commits Warning */}
          {showDependentWarning && (
            <div
              data-testid="dependent-commits-warning"
              className="flex items-start gap-2 px-3 py-2.5 rounded-lg"
              style={{
                backgroundColor: "rgba(245, 158, 11, 0.1)",
                color: "var(--status-warning)",
              }}
            >
              <span className="mt-0.5 flex-shrink-0">
                <WarningIcon />
              </span>
              <span className="text-sm">
                Warning: Other commits depend on this one. Reverting may cause
                conflicts or break code that was built on top of these changes.
              </span>
            </div>
          )}

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
              backgroundColor: isProcessing
                ? "var(--bg-hover)"
                : "var(--accent-primary)",
              color: isProcessing ? "var(--text-muted)" : "#fff",
              cursor: isProcessing ? "not-allowed" : "pointer",
            }}
          >
            {isProcessing ? "Processing..." : "Confirm Re-run"}
          </button>
        </div>
      </div>
    </div>
  );
}
