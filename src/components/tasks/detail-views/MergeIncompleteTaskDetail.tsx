/**
 * MergeIncompleteTaskDetail - View for tasks in merge_incomplete state
 *
 * Shows error context for non-conflict git failures (branch deleted, git lock,
 * network failure), recovery steps, and action buttons for retry/resolve.
 * Uses error variant (red) to distinguish from MergeConflict's warning (amber).
 */

import { useState, useCallback } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  Loader2,
  RefreshCw,
  SkipForward,
  Clock,
  PlayCircle,
  XCircle,
  CheckCircle,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { ValidationProgress } from "./MergingTaskDetail";
import type { Task, TaskMetadata, MergeRecoveryEvent } from "@/types/task";
import { useQueryClient } from "@tanstack/react-query";
import { taskKeys } from "@/hooks/useTasks";
import { extractErrorMessage } from "@/lib/errors";

interface MergeIncompleteTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
}

interface MergeErrorContext {
  error: string | null;
  sourceBranch: string | null;
  targetBranch: string | null;
  diagnosticInfo: string | null;
  hasValidationFailures: boolean;
  recoveryEvents: MergeRecoveryEvent[];
  metadata: TaskMetadata | null;
}

function parseMergeError(metadata?: string | null): MergeErrorContext | null {
  if (!metadata) return null;
  try {
    const m: TaskMetadata = JSON.parse(metadata);
    return {
      error: m.error ?? null,
      sourceBranch: m.source_branch ?? null,
      targetBranch: m.target_branch ?? null,
      diagnosticInfo: m.diagnostic_info ?? null,
      hasValidationFailures: Array.isArray(m.validation_failures) && m.validation_failures.length > 0,
      recoveryEvents: m.merge_recovery?.events ?? [],
      metadata: m,
    };
  } catch {
    return null;
  }
}

/**
 * ErrorContextCard - Shows actual error details or generic fallback
 */
function ErrorContextCard({ mergeError }: { mergeError: MergeErrorContext | null }) {
  if (!mergeError) {
    return (
      <div className="space-y-3">
        <p className="text-[13px] text-white/60">
          The merge failed due to a git error that is not a merge conflict.
          This can happen when:
        </p>
        <ul className="list-disc list-inside space-y-1.5 text-[13px] text-white/50">
          <li>The task branch was deleted or corrupted</li>
          <li>A git lock file is preventing operations</li>
          <li>Network issues interrupted a fetch operation</li>
          <li>The worktree directory is missing or inaccessible</li>
        </ul>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {mergeError.error && (
        <div
          className="rounded-md px-3 py-2 font-mono text-[12px] text-white/80 whitespace-pre-wrap"
          style={{ backgroundColor: "rgba(255, 69, 58, 0.10)" }}
        >
          {mergeError.error}
        </div>
      )}
      {(mergeError.sourceBranch || mergeError.targetBranch) && (
        <div className="flex items-center gap-2 text-[13px] text-white/60">
          <span className="font-mono text-white/70">{mergeError.sourceBranch ?? "unknown"}</span>
          <span className="text-white/40">&rarr;</span>
          <span className="font-mono text-white/70">{mergeError.targetBranch ?? "unknown"}</span>
        </div>
      )}
      {mergeError.diagnosticInfo && (
        <div className="text-[12px] text-white/50 whitespace-pre-wrap">
          {mergeError.diagnosticInfo}
        </div>
      )}
    </div>
  );
}

/**
 * RecoverySteps - Numbered steps for manual recovery
 */
function RecoverySteps({ branchName, targetBranch, hasValidationFailures }: { branchName: string; targetBranch?: string | null; hasValidationFailures: boolean }) {
  return (
    <div className="space-y-3">
      {hasValidationFailures ? (
        <>
          <p className="text-[13px] text-white/60">
            Your validation commands (build, type checks, linting) failed,
            so the merge could not be completed. To recover:
          </p>
          <ol className="list-decimal list-inside space-y-2 text-[13px] text-white/50">
            <li>
              Fix the build, type, or lint errors in your codebase
            </li>
            <li>
              Click <strong className="text-white/70">Retry Merge</strong> to
              re-run validation and complete the merge
            </li>
            <li>
              Click{" "}
              <strong className="text-white/70">Retry (Skip Validation)</strong>{" "}
              to complete the merge without running validation
            </li>
            <li>
              If fixed manually, click{" "}
              <strong className="text-white/70">Mark Resolved</strong>
            </li>
          </ol>
        </>
      ) : (
        <>
          <p className="text-[13px] text-white/60">
            To recover, try the following steps:
          </p>
          <ol className="list-decimal list-inside space-y-2 text-[13px] text-white/50">
            <li>
              Check if the branch exists:{" "}
              <code className="text-white/70 bg-white/5 px-1 rounded">
                git branch --list {branchName}
              </code>
            </li>
            <li>
              Remove any stale lock files:{" "}
              <code className="text-white/70 bg-white/5 px-1 rounded">
                rm -f .git/index.lock
              </code>
            </li>
            <li>
              Click <strong className="text-white/70">Retry Merge</strong> to
              attempt the merge again
            </li>
            <li>
              If the issue is resolved manually, click{" "}
              <strong className="text-white/70">Mark Resolved</strong>
            </li>
          </ol>
        </>
      )}
      <div className="flex gap-4 pt-2">
        <div>
          <span className="text-[11px] text-white/40">Source: </span>
          <span className="text-[11px] text-white/60 font-mono">
            {branchName}
          </span>
        </div>
        {targetBranch && (
          <div>
            <span className="text-[11px] text-white/40">Target: </span>
            <span className="text-[11px] text-white/60 font-mono">
              {targetBranch}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * RecoveryTimeline - Shows chronological timeline of merge recovery attempts
 */
function RecoveryTimeline({ events }: { events: MergeRecoveryEvent[] }) {
  const formatTimestamp = (isoString: string) => {
    try {
      const date = new Date(isoString);
      return date.toLocaleString("en-US", {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
    } catch {
      return isoString;
    }
  };

  const getEventIcon = (kind: string) => {
    switch (kind) {
      case "deferred":
        return Clock;
      case "auto_retry_triggered":
        return RefreshCw;
      case "attempt_started":
        return PlayCircle;
      case "attempt_failed":
        return XCircle;
      case "attempt_succeeded":
        return CheckCircle;
      case "manual_retry":
        return RefreshCw;
      default:
        return Clock;
    }
  };

  const getEventColor = (kind: string) => {
    switch (kind) {
      case "deferred":
        return "rgba(255, 159, 10, 0.7)"; // amber
      case "auto_retry_triggered":
      case "manual_retry":
        return "rgba(255, 107, 53, 0.7)"; // orange
      case "attempt_started":
        return "rgba(100, 200, 255, 0.7)"; // blue
      case "attempt_failed":
        return "rgba(255, 69, 58, 0.7)"; // red
      case "attempt_succeeded":
        return "#34c759"; // green
      default:
        return "rgba(255, 255, 255, 0.5)"; // white/gray
    }
  };

  const getKindLabel = (kind: string) => {
    switch (kind) {
      case "deferred":
        return "Deferred";
      case "auto_retry_triggered":
        return "Auto-retry Triggered";
      case "attempt_started":
        return "Attempt Started";
      case "attempt_failed":
        return "Attempt Failed";
      case "attempt_succeeded":
        return "Succeeded";
      case "manual_retry":
        return "Manual Retry";
      default:
        return kind;
    }
  };

  const getSourceBadge = (source: string) => {
    const colors = {
      system: "rgba(100, 200, 255, 0.15)",
      auto: "rgba(255, 107, 53, 0.15)",
      user: "rgba(52, 199, 89, 0.15)",
    };
    return (
      <span
        className="text-[10px] px-2 py-0.5 rounded-full font-medium uppercase tracking-wide"
        style={{
          backgroundColor: colors[source as keyof typeof colors] ?? "rgba(255, 255, 255, 0.1)",
          color: "rgba(255, 255, 255, 0.7)",
        }}
      >
        {source}
      </span>
    );
  };

  return (
    <div className="space-y-3">
      {events.map((event, idx) => {
        const Icon = getEventIcon(event.kind);
        const color = getEventColor(event.kind);

        return (
          <div
            key={idx}
            className="flex gap-3 pb-3"
            style={{
              borderBottom:
                idx < events.length - 1
                  ? "1px solid rgba(255, 255, 255, 0.08)"
                  : "none",
            }}
          >
            {/* Icon */}
            <div
              className="flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center"
              style={{ backgroundColor: `${color}20` }}
            >
              <Icon className="w-4 h-4" style={{ color }} />
            </div>

            {/* Content */}
            <div className="flex-1 space-y-1.5">
              {/* Header: kind + timestamp */}
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 flex-wrap">
                  <span className="text-[13px] font-medium text-white/90">
                    {getKindLabel(event.kind)}
                  </span>
                  {getSourceBadge(event.source)}
                  {event.attempt !== undefined && (
                    <span className="text-[11px] text-white/40">
                      Attempt #{event.attempt}
                    </span>
                  )}
                </div>
                <span className="text-[11px] text-white/40 font-mono">
                  {formatTimestamp(event.at)}
                </span>
              </div>

              {/* Message */}
              <p className="text-[13px] text-white/70">{event.message}</p>

              {/* Additional metadata */}
              <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-white/50">
                {event.blocking_task_id && (
                  <div>
                    <span className="text-white/40">Blocker: </span>
                    <span className="font-mono">{event.blocking_task_id.slice(0, 8)}</span>
                  </div>
                )}
                {event.target_branch && (
                  <div>
                    <span className="text-white/40">Target: </span>
                    <span className="font-mono">{event.target_branch}</span>
                  </div>
                )}
                {event.reason_code && (
                  <div>
                    <span className="text-white/40">Reason: </span>
                    <span>{event.reason_code.replace(/_/g, " ")}</span>
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

/**
 * RecoveryBadges - Show status badges based on recovery state
 */
function RecoveryBadges({
  hasAutoRetry,
  hasDeferred,
  lastAttemptFailed,
}: {
  hasAutoRetry: boolean;
  hasDeferred: boolean;
  lastAttemptFailed: boolean;
}) {
  return (
    <div className="flex gap-2 flex-wrap">
      {hasAutoRetry && (
        <span
          className="text-[11px] px-2.5 py-1 rounded-full font-medium"
          style={{
            backgroundColor: "rgba(255, 107, 53, 0.15)",
            color: "rgba(255, 107, 53, 0.9)",
          }}
        >
          Auto-recovery attempted
        </span>
      )}
      {hasDeferred && (
        <span
          className="text-[11px] px-2.5 py-1 rounded-full font-medium"
          style={{
            backgroundColor: "rgba(255, 159, 10, 0.15)",
            color: "rgba(255, 159, 10, 0.9)",
          }}
        >
          Deferred due to active merge
        </span>
      )}
      {lastAttemptFailed && (
        <span
          className="text-[11px] px-2.5 py-1 rounded-full font-medium"
          style={{
            backgroundColor: "rgba(255, 69, 58, 0.15)",
            color: "rgba(255, 69, 58, 0.9)",
          }}
        >
          Last attempt failed
        </span>
      )}
    </div>
  );
}

/**
 * ActionButtons - Retry Merge (primary) + Mark Resolved (green)
 */
function ActionButtons({
  onRetry,
  onRetrySkipValidation,
  onResolve,
  isProcessing,
}: {
  onRetry: () => void;
  onRetrySkipValidation?: (() => void) | undefined;
  onResolve: () => void;
  isProcessing: boolean;
}) {
  return (
    <div className="flex gap-2 justify-end flex-wrap">
      <Button
        data-testid="retry-merge-button"
        onClick={onRetry}
        disabled={isProcessing}
        className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
        style={{
          color: "white",
          backgroundColor: "#ff6b35",
        }}
      >
        {isProcessing ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <RefreshCw className="w-4 h-4" />
        )}
        Retry Merge
      </Button>
      {onRetrySkipValidation && (
        <Button
          data-testid="retry-skip-validation-button"
          onClick={onRetrySkipValidation}
          disabled={isProcessing}
          className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
          style={{
            color: "white",
            backgroundColor: "rgba(255, 159, 10, 0.85)",
          }}
        >
          {isProcessing ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <SkipForward className="w-4 h-4" />
          )}
          Retry (Skip Validation)
        </Button>
      )}
      <Button
        data-testid="resolve-merge-button"
        onClick={onResolve}
        disabled={isProcessing}
        className="h-9 px-4 gap-2 rounded-lg font-medium text-[13px]"
        style={{
          color: "white",
          backgroundColor: "#34c759",
        }}
      >
        {isProcessing ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <CheckCircle2 className="w-4 h-4" />
        )}
        Mark Resolved
      </Button>
    </div>
  );
}

export function MergeIncompleteTaskDetail({
  task,
  isHistorical = false,
}: MergeIncompleteTaskDetailProps) {
  const queryClient = useQueryClient();
  const [isProcessing, setIsProcessing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mergeError = parseMergeError(task.metadata);
  const branchName = mergeError?.sourceBranch ?? task.taskBranch ?? "task branch";

  // Derive recovery state badges from events
  const hasAutoRetryAttempts = mergeError?.recoveryEvents.some(
    (e) => e.kind === "auto_retry_triggered" || e.source === "auto"
  );
  const hasDeferred = mergeError?.recoveryEvents.some((e) => e.kind === "deferred");
  const lastEvent = mergeError?.recoveryEvents[mergeError.recoveryEvents.length - 1];
  const lastAttemptFailed = lastEvent?.kind === "attempt_failed";

  const handleRetryMerge = useCallback(async () => {
    setIsProcessing(true);
    setError(null);
    try {
      await invoke("retry_merge", { taskId: task.id });
      await queryClient.invalidateQueries({
        queryKey: taskKeys.list(task.projectId),
      });
    } catch (err) {
      setError(extractErrorMessage(err, "Failed to retry merge"));
    } finally {
      setIsProcessing(false);
    }
  }, [task.id, task.projectId, queryClient]);

  const handleRetrySkipValidation = useCallback(async () => {
    setIsProcessing(true);
    setError(null);
    try {
      await invoke("retry_merge", { taskId: task.id, skipValidation: true });
      await queryClient.invalidateQueries({
        queryKey: taskKeys.list(task.projectId),
      });
    } catch (err) {
      setError(
        extractErrorMessage(err, "Failed to retry merge (skipping validation)"),
      );
    } finally {
      setIsProcessing(false);
    }
  }, [task.id, task.projectId, queryClient]);

  const handleMarkResolved = useCallback(async () => {
    setIsProcessing(true);
    setError(null);
    try {
      await invoke("resolve_merge_conflict", { taskId: task.id });
      await queryClient.invalidateQueries({
        queryKey: taskKeys.list(task.projectId),
      });
    } catch (err) {
      setError(
        extractErrorMessage(err, "Failed to mark merge as resolved"),
      );
    } finally {
      setIsProcessing(false);
    }
  }, [task.id, task.projectId, queryClient]);

  return (
    <TwoColumnLayout
      description={task.description}
      testId="merge-incomplete-task-detail"
    >
      {/* Status Banner - error (red) variant */}
      <StatusBanner
        icon={AlertTriangle}
        title="Merge Incomplete"
        subtitle={
          mergeError?.hasValidationFailures
            ? isHistorical
              ? "Merge validation failed"
              : "Merge validation failed — action required"
            : isHistorical
              ? "A git error prevented the merge"
              : "A git error prevented the merge — action required"
        }
        variant="error"
        badge={
          <StatusPill
            icon={AlertTriangle}
            label="Error"
            variant="error"
            size="md"
          />
        }
      />

      {/* Recovery Attempts Timeline or Fallback */}
      <section data-testid="recovery-attempts-section">
        <SectionTitle>Recovery Attempts</SectionTitle>
        {mergeError && mergeError.recoveryEvents.length > 0 ? (
          <>
            <DetailCard>
              <RecoveryTimeline events={mergeError.recoveryEvents} />
            </DetailCard>
            {/* Recovery Status Badges */}
            <div className="mt-3">
              <RecoveryBadges
                hasAutoRetry={hasAutoRetryAttempts ?? false}
                hasDeferred={hasDeferred ?? false}
                lastAttemptFailed={lastAttemptFailed ?? false}
              />
            </div>
          </>
        ) : (
          <DetailCard>
            <p className="text-[13px] text-white/50 italic">
              No recorded recovery attempts for this task.
            </p>
          </DetailCard>
        )}
      </section>

      {/* Error Context */}
      <section data-testid="error-context-section">
        <SectionTitle>What Happened</SectionTitle>
        <DetailCard variant="error">
          <ErrorContextCard mergeError={mergeError} />
        </DetailCard>
      </section>

      {/* Validation Log (when failure was caused by validation) */}
      {mergeError?.hasValidationFailures && (
        <ValidationProgress
          taskId={task.id}
          metadata={task.metadata}
        />
      )}

      {/* Recovery Steps (not in historical mode) */}
      {!isHistorical && (
        <section data-testid="recovery-steps-section">
          <SectionTitle>How to Recover</SectionTitle>
          <DetailCard>
            <RecoverySteps branchName={branchName} targetBranch={mergeError?.targetBranch ?? null} hasValidationFailures={mergeError?.hasValidationFailures ?? false} />
          </DetailCard>
        </section>
      )}

      {/* Error Display */}
      {error && (
        <div
          className="p-3 rounded-lg text-[13px]"
          style={{
            backgroundColor: "rgba(255, 69, 58, 0.12)",
            color: "#ff6961",
          }}
        >
          {error}
        </div>
      )}

      {/* Actions (hidden in historical mode) */}
      {!isHistorical && (
        <section data-testid="action-buttons">
          <ActionButtons
            onRetry={handleRetryMerge}
            onRetrySkipValidation={mergeError?.hasValidationFailures ? handleRetrySkipValidation : undefined}
            onResolve={handleMarkResolved}
            isProcessing={isProcessing}
          />
        </section>
      )}
    </TwoColumnLayout>
  );
}
