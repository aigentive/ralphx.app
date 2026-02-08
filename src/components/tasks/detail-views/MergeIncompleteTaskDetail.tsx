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
import type { Task } from "@/types/task";
import { useQueryClient } from "@tanstack/react-query";
import { taskKeys } from "@/hooks/useTasks";

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
}

function parseMergeError(metadata?: string | null): MergeErrorContext | null {
  if (!metadata) return null;
  try {
    const m = JSON.parse(metadata);
    return {
      error: m.error ?? null,
      sourceBranch: m.source_branch ?? null,
      targetBranch: m.target_branch ?? null,
      diagnosticInfo: m.diagnostic_info ?? null,
      hasValidationFailures: Array.isArray(m.validation_failures) && m.validation_failures.length > 0,
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
function RecoverySteps({ branchName, targetBranch }: { branchName: string; targetBranch?: string | null }) {
  return (
    <div className="space-y-3">
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

  const handleRetryMerge = useCallback(async () => {
    setIsProcessing(true);
    setError(null);
    try {
      await invoke("retry_merge", { taskId: task.id });
      await queryClient.invalidateQueries({
        queryKey: taskKeys.list(task.projectId),
      });
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to retry merge",
      );
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
        err instanceof Error ? err.message : "Failed to retry merge",
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
        err instanceof Error
          ? err.message
          : "Failed to mark merge as resolved",
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
          isHistorical
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
            <RecoverySteps branchName={branchName} targetBranch={mergeError?.targetBranch ?? null} />
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
