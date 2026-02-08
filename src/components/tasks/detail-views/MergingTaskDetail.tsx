/**
 * MergingTaskDetail - Handles both pending_merge and merging states
 *
 * pending_merge: Programmatic merge attempt in progress (Phase 1)
 * merging: Agent-assisted conflict resolution in progress (Phase 2)
 *
 * This combined view provides a seamless UX during the merge process.
 * PendingMerge is typically very brief (1-3 seconds).
 */

import { useState, useMemo } from "react";
import {
  Loader2,
  GitMerge,
  AlertTriangle,
  Bot,
  FileWarning,
  CheckCircle2,
  XCircle,
  ChevronDown,
  ChevronRight,
  Clock,
  Wrench,
  Terminal,
} from "lucide-react";
import {
  SectionTitle,
  DetailCard,
  StatusBanner,
  StatusPill,
  TwoColumnLayout,
} from "./shared";
import { useMergeValidationEvents } from "@/hooks/useMergeValidationEvents";
import type { Task } from "@/types/task";
import type { MergeValidationStepEvent } from "@/types/events";

interface MergingTaskDetailProps {
  task: Task;
  isHistorical?: boolean;
  viewStatus?: string | undefined;
}

/**
 * ConflictFilesList - Shows files with merge conflicts
 */
function ConflictFilesList({ files }: { files: string[] }) {
  if (files.length === 0) return null;

  return (
    <div className="space-y-2">
      {files.map((file, index) => (
        <div
          key={index}
          className="flex items-center gap-2 py-2 px-3 rounded-lg"
          style={{ backgroundColor: "rgba(255, 159, 10, 0.08)" }}
        >
          <FileWarning className="w-4 h-4" style={{ color: "#ff9f0a" }} />
          <span
            className="text-[13px] font-mono text-white/70 truncate"
            title={file}
          >
            {file}
          </span>
        </div>
      ))}
    </div>
  );
}

interface ValidationFailureEntry {
  command: string;
  path?: string;
  exit_code?: number;
  stderr?: string;
}

/**
 * Parse validation recovery state from task metadata.
 */
function parseValidationRecovery(metadata: string | Record<string, unknown> | null | undefined): {
  isRecovery: boolean;
  failures: ValidationFailureEntry[];
} {
  if (!metadata) return { isRecovery: false, failures: [] };
  try {
    const parsed = typeof metadata === "string" ? JSON.parse(metadata) : metadata;
    const isRecovery = parsed?.validation_recovery === true;
    if (!isRecovery) return { isRecovery: false, failures: [] };
    const failures = Array.isArray(parsed?.validation_failures)
      ? (parsed.validation_failures as ValidationFailureEntry[])
      : [];
    return { isRecovery: true, failures };
  } catch {
    return { isRecovery: false, failures: [] };
  }
}

/**
 * ValidationFailuresList - Shows validation command failures that triggered recovery
 */
function ValidationFailuresList({ failures }: { failures: ValidationFailureEntry[] }) {
  const [expandedIndex, setExpandedIndex] = useState<number | null>(null);

  if (failures.length === 0) return null;

  return (
    <div className="space-y-1.5">
      {failures.map((failure, index) => {
        const hasStderr = failure.stderr && failure.stderr.trim().length > 0;
        const expanded = expandedIndex === index;

        return (
          <div
            key={index}
            className="rounded-lg overflow-hidden"
            style={{ backgroundColor: "rgba(255, 69, 58, 0.08)" }}
          >
            <button
              type="button"
              className="w-full flex items-center gap-2 py-2 px-3 text-left"
              onClick={() => hasStderr && setExpandedIndex(expanded ? null : index)}
              style={{ cursor: hasStderr ? "pointer" : "default" }}
            >
              <XCircle className="w-4 h-4 shrink-0" style={{ color: "#ff453a" }} />
              <Terminal className="w-3.5 h-3.5 shrink-0 text-white/30" />
              <span className="text-[12px] font-mono text-white/70 truncate flex-1" title={failure.command}>
                {failure.command}
              </span>
              {failure.exit_code != null && (
                <span className="text-[10px] text-white/30 shrink-0">exit {failure.exit_code}</span>
              )}
              {hasStderr && (
                expanded
                  ? <ChevronDown className="w-3.5 h-3.5 text-white/30 shrink-0" />
                  : <ChevronRight className="w-3.5 h-3.5 text-white/30 shrink-0" />
              )}
            </button>
            {expanded && hasStderr && (
              <div
                className="px-3 pb-3 max-h-[200px] overflow-y-auto"
                style={{ scrollbarWidth: "thin" }}
              >
                <pre className="text-[11px] font-mono whitespace-pre-wrap break-all leading-relaxed" style={{ color: "#ff6961" }}>
                  {failure.stderr}
                </pre>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

/**
 * MergeProgressSteps - Shows progress through merge phases
 */
function MergeProgressSteps({
  isProgrammaticPhase,
  isHistorical,
  historicalMode,
  isValidationRecovery,
}: {
  isProgrammaticPhase: boolean;
  isHistorical?: boolean | undefined;
  historicalMode?: "attempted" | "resolving" | undefined;
  isValidationRecovery?: boolean;
}) {
  // Validation recovery mode: different steps (no conflict resolution)
  if (isValidationRecovery && !isProgrammaticPhase) {
    const agentStepStatus = isHistorical ? ("completed" as const) : ("active" as const);
    const recoverySteps = [
      { label: "Merge completed", status: "completed" as const },
      { label: "Merge validation failed", status: "completed" as const },
      { label: "AI agent fixing build errors", status: agentStepStatus },
      { label: "Re-validating fixes", status: "pending" as const },
    ];

    return (
      <div className="divide-y divide-white/5">
        {recoverySteps.map((step, index) => (
          <div key={index} className="flex items-center gap-3 py-2.5">
            <div className="relative">
              {step.status === "completed" && (
                <CheckCircle2 className="w-5 h-5" style={{ color: "#34c759" }} />
              )}
              {step.status === "active" && !isHistorical && (
                <div className="relative">
                  <Loader2
                    className="w-5 h-5 animate-spin"
                    style={{ color: "#ff6b35" }}
                  />
                  <div
                    className="absolute inset-0 rounded-full animate-pulse"
                    style={{
                      background: "radial-gradient(circle, rgba(255,107,53,0.3) 0%, transparent 70%)",
                    }}
                  />
                </div>
              )}
              {step.status === "active" && isHistorical && (
                <div
                  className="w-5 h-5 rounded-full border-2"
                  style={{
                    borderColor: "rgba(255,255,255,0.2)",
                    backgroundColor: "rgba(255, 107, 53, 0.35)",
                  }}
                />
              )}
              {step.status === "pending" && (
                <div
                  className="w-5 h-5 rounded-full border-2"
                  style={{ borderColor: "rgba(255,255,255,0.2)" }}
                />
              )}
            </div>
            <span
              className="text-[13px] font-medium"
              style={{
                color:
                  step.status === "completed"
                    ? "rgba(255,255,255,0.6)"
                    : step.status === "active"
                    ? isHistorical
                      ? "rgba(255,255,255,0.35)"
                      : "#ff6b35"
                    : "rgba(255,255,255,0.35)",
              }}
            >
              {step.label}
            </span>
          </div>
        ))}
      </div>
    );
  }

  const steps = isHistorical
    ? historicalMode === "attempted"
      ? ([
          { label: "Fetching latest changes", status: "completed" },
          { label: "Rebasing onto base branch", status: "completed" },
          { label: "Resolving conflicts", status: "pending" },
          { label: "Completing merge", status: "pending" },
        ] as const)
      : ([
          { label: "Fetching latest changes", status: "completed" },
          { label: "Rebasing onto base branch", status: "completed" },
          { label: "Resolving conflicts", status: "active" },
          { label: "Completing merge", status: "pending" },
        ] as const)
    : ([
        {
          label: "Fetching latest changes",
          status: isProgrammaticPhase ? "active" : "completed",
        },
        {
          label: "Rebasing onto base branch",
          status: isProgrammaticPhase ? "pending" : "completed",
        },
        {
          label: "Resolving conflicts",
          status: isProgrammaticPhase ? "pending" : "active",
        },
        {
          label: "Completing merge",
          status: "pending",
        },
      ] as const);

  return (
    <div className="divide-y divide-white/5">
      {steps.map((step, index) => (
        <div key={index} className="flex items-center gap-3 py-2.5">
          <div className="relative">
            {step.status === "completed" && (
              <CheckCircle2 className="w-5 h-5" style={{ color: "#34c759" }} />
            )}
            {step.status === "active" && !isHistorical && (
              <div className="relative">
                <Loader2
                  className="w-5 h-5 animate-spin"
                  style={{ color: "#0a84ff" }}
                />
                <div
                  className="absolute inset-0 rounded-full animate-pulse"
                  style={{
                    background: "radial-gradient(circle, rgba(10,132,255,0.3) 0%, transparent 70%)",
                  }}
                />
              </div>
            )}
            {step.status === "active" && isHistorical && (
              <div
                className="w-5 h-5 rounded-full border-2"
                style={{
                  borderColor: "rgba(255,255,255,0.2)",
                  backgroundColor: "rgba(255, 159, 10, 0.35)",
                }}
              />
            )}
            {step.status === "pending" && (
              <div
                className="w-5 h-5 rounded-full border-2"
                style={{ borderColor: "rgba(255,255,255,0.2)" }}
              />
            )}
          </div>
          <span
            className="text-[13px] font-medium"
            style={{
              color:
                step.status === "completed"
                  ? "rgba(255,255,255,0.6)"
                  : step.status === "active"
                  ? isHistorical
                    ? "rgba(255,255,255,0.35)"
                    : "#64d2ff"
                  : "rgba(255,255,255,0.35)",
            }}
          >
            {step.label}
          </span>
        </div>
      ))}
    </div>
  );
}

function formatDuration(ms: number): string {
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
}

/**
 * ValidationStepRow - Single validation command row with collapsible output
 */
function ValidationStepRow({ step }: { step: MergeValidationStepEvent }) {
  const isRunning = step.status === "running";
  const isFailed = step.status === "failed";
  const hasOutput = (step.stdout && step.stdout.trim().length > 0) ||
    (step.stderr && step.stderr.trim().length > 0);
  const [expanded, setExpanded] = useState(isRunning || isFailed);

  const statusIcon = isRunning ? (
    <Loader2 className="w-4 h-4 animate-spin" style={{ color: "#0a84ff" }} />
  ) : isFailed ? (
    <XCircle className="w-4 h-4" style={{ color: "#ff453a" }} />
  ) : (
    <CheckCircle2 className="w-4 h-4" style={{ color: "#34c759" }} />
  );

  return (
    <div
      className="rounded-lg overflow-hidden"
      style={{ backgroundColor: "rgba(0, 0, 0, 0.3)" }}
    >
      <button
        type="button"
        className="w-full flex items-center gap-2.5 px-3 py-2.5 text-left"
        onClick={() => hasOutput && setExpanded(!expanded)}
        style={{ cursor: hasOutput ? "pointer" : "default" }}
      >
        {statusIcon}
        <span
          className="text-[10px] uppercase font-semibold tracking-wider px-1.5 py-0.5 rounded"
          style={{ backgroundColor: "rgba(255, 107, 53, 0.15)", color: "#ff6b35" }}
        >
          validate
        </span>
        <span className="text-[12px] text-white/80 font-mono truncate flex-1" title={step.label}>
          {step.label}
        </span>
        {step.duration_ms != null && (
          <span className="flex items-center gap-1 text-[11px] text-white/40 shrink-0">
            <Clock className="w-3 h-3" />
            {formatDuration(step.duration_ms)}
          </span>
        )}
        {hasOutput && (
          expanded
            ? <ChevronDown className="w-3.5 h-3.5 text-white/30 shrink-0" />
            : <ChevronRight className="w-3.5 h-3.5 text-white/30 shrink-0" />
        )}
      </button>
      {expanded && hasOutput && (
        <div
          className="px-3 pb-3 max-h-[200px] overflow-y-auto"
          style={{ scrollbarWidth: "thin" }}
        >
          {step.stdout && step.stdout.trim() && (
            <pre className="text-[11px] font-mono text-white/50 whitespace-pre-wrap break-all leading-relaxed">
              {step.stdout}
            </pre>
          )}
          {step.stderr && step.stderr.trim() && (
            <pre className="text-[11px] font-mono whitespace-pre-wrap break-all leading-relaxed" style={{ color: "#ff6961" }}>
              {step.stderr}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * SetupPhaseGroup - Collapses all setup steps into a single row.
 * Shows summary when collapsed, individual commands when expanded.
 */
function SetupPhaseGroup({ steps }: { steps: MergeValidationStepEvent[] }) {
  const anyFailed = steps.some((s) => s.status === "failed");
  const anyRunning = steps.some((s) => s.status === "running");
  const [expanded, setExpanded] = useState(anyFailed);

  const totalMs = steps.reduce((sum, s) => sum + (s.duration_ms ?? 0), 0);

  const statusIcon = anyRunning ? (
    <Loader2 className="w-4 h-4 animate-spin" style={{ color: "#0a84ff" }} />
  ) : anyFailed ? (
    <XCircle className="w-4 h-4" style={{ color: "#ff453a" }} />
  ) : (
    <CheckCircle2 className="w-4 h-4" style={{ color: "#34c759" }} />
  );

  return (
    <div
      className="rounded-lg overflow-hidden"
      style={{ backgroundColor: "rgba(0, 0, 0, 0.3)" }}
    >
      <button
        type="button"
        className="w-full flex items-center gap-2.5 px-3 py-2.5 text-left cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        {statusIcon}
        <span
          className="text-[10px] uppercase font-semibold tracking-wider px-1.5 py-0.5 rounded"
          style={{ backgroundColor: "rgba(10, 132, 255, 0.15)", color: "#64d2ff" }}
        >
          setup
        </span>
        <span className="text-[12px] text-white/80 flex-1">
          {steps.length} command{steps.length !== 1 ? "s" : ""}
        </span>
        {totalMs > 0 && (
          <span className="flex items-center gap-1 text-[11px] text-white/40 shrink-0">
            <Clock className="w-3 h-3" />
            {formatDuration(totalMs)}
          </span>
        )}
        {expanded
          ? <ChevronDown className="w-3.5 h-3.5 text-white/30 shrink-0" />
          : <ChevronRight className="w-3.5 h-3.5 text-white/30 shrink-0" />}
      </button>
      {expanded && (
        <div className="px-3 pb-2.5 space-y-2">
          {steps.map((step, i) => {
            const isFailed = step.status === "failed";
            const isRunning = step.status === "running";
            const hasOutput = (step.stdout && step.stdout.trim().length > 0) ||
              (step.stderr && step.stderr.trim().length > 0);
            return (
              <div key={`setup-${step.command}-${i}`} className="space-y-1">
                <div className="flex items-center gap-2">
                  {isRunning ? (
                    <Loader2 className="w-3 h-3 animate-spin" style={{ color: "#0a84ff" }} />
                  ) : isFailed ? (
                    <XCircle className="w-3 h-3" style={{ color: "#ff453a" }} />
                  ) : (
                    <CheckCircle2 className="w-3 h-3" style={{ color: "#34c759" }} />
                  )}
                  <span className="text-[11px] text-white/60 truncate flex-1" title={step.label}>
                    {step.label}
                  </span>
                  {step.duration_ms != null && (
                    <span className="text-[10px] text-white/30 shrink-0">
                      {formatDuration(step.duration_ms)}
                    </span>
                  )}
                </div>
                <code className="block text-[10px] font-mono text-white/40 pl-5 truncate" title={step.command}>
                  $ {step.command}
                </code>
                {hasOutput && (
                  <div
                    className="pl-5 max-h-[120px] overflow-y-auto"
                    style={{ scrollbarWidth: "thin" }}
                  >
                    {step.stdout && step.stdout.trim() && (
                      <pre className="text-[10px] font-mono text-white/40 whitespace-pre-wrap break-all leading-relaxed">
                        {step.stdout}
                      </pre>
                    )}
                    {step.stderr && step.stderr.trim() && (
                      <pre className="text-[10px] font-mono whitespace-pre-wrap break-all leading-relaxed" style={{ color: "#ff6961" }}>
                        {step.stderr}
                      </pre>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/**
 * Parse validation log entries from task metadata (historical data).
 * Returns entries matching MergeValidationStepEvent shape.
 */
function parseMetadataValidationLog(metadata: string | Record<string, unknown> | null | undefined): MergeValidationStepEvent[] {
  if (!metadata) return [];
  try {
    const parsed = typeof metadata === "string" ? JSON.parse(metadata) : metadata;
    const log = parsed?.validation_log;
    if (!Array.isArray(log)) return [];
    return log.map((entry: Record<string, unknown>) => ({
      task_id: String(entry.task_id ?? ""),
      phase: (entry.phase === "setup" || entry.phase === "validate") ? entry.phase : "validate",
      command: String(entry.command ?? ""),
      path: String(entry.path ?? ""),
      label: String(entry.label ?? entry.command ?? ""),
      status: (entry.status === "running" || entry.status === "success" || entry.status === "failed")
        ? entry.status
        : "success",
      exit_code: typeof entry.exit_code === "number" ? entry.exit_code : null,
      stdout: typeof entry.stdout === "string" ? entry.stdout : undefined,
      stderr: typeof entry.stderr === "string" ? entry.stderr : undefined,
      duration_ms: typeof entry.duration_ms === "number" ? entry.duration_ms : undefined,
    })) as MergeValidationStepEvent[];
  } catch {
    return [];
  }
}

/**
 * ValidationProgress - Shows real-time or historical validation command progress
 *
 * Data sources:
 * - Live: useMergeValidationEvents(task.id) — real-time events during pending_merge
 * - Historical: task.metadata.validation_log — stored after merge attempt
 * - Merge: prefer live events; fall back to metadata if live is empty
 */
export function ValidationProgress({
  taskId,
  metadata,
  liveSteps,
}: {
  taskId: string;
  metadata?: string | Record<string, unknown> | null | undefined;
  liveSteps?: MergeValidationStepEvent[] | undefined;
}) {
  const metadataSteps = useMemo(() => parseMetadataValidationLog(metadata), [metadata]);
  const steps = liveSteps && liveSteps.length > 0 ? liveSteps : metadataSteps;

  if (steps.length === 0) return null;

  const source = liveSteps && liveSteps.length > 0 ? "live" : "historical";
  const setupSteps = steps.filter((s) => s.phase === "setup");
  const validateSteps = steps.filter((s) => s.phase !== "setup");

  return (
    <section data-testid={`validation-progress-${taskId}`}>
      <SectionTitle>
        Merge Validation
        {source === "live" && (
          <span className="ml-2 text-[10px] font-normal text-white/30">(live)</span>
        )}
      </SectionTitle>
      <div className="space-y-1.5">
        {setupSteps.length > 0 && <SetupPhaseGroup steps={setupSteps} />}
        {validateSteps.map((step, index) => (
          <ValidationStepRow key={`${step.phase}-${step.command}-${index}`} step={step} />
        ))}
      </div>
    </section>
  );
}

export function MergingTaskDetail({ task, isHistorical, viewStatus }: MergingTaskDetailProps) {
  const status = isHistorical && viewStatus ? viewStatus : task.internalStatus;
  const isProgrammaticPhase = status === "pending_merge";
  const isAgentPhase = status === "merging";
  const historicalOutcome = isHistorical
    ? task.internalStatus === "merged" || task.internalStatus === "merge_conflict"
      ? task.internalStatus
      : null
    : null;
  const historicalMode = isHistorical
    ? isProgrammaticPhase
      ? "attempted"
      : "resolving"
    : undefined;

  // Detect validation recovery mode from task metadata
  const { isRecovery: isValidationRecovery, failures: validationFailures } = useMemo(
    () => parseValidationRecovery(task.metadata),
    [task.metadata],
  );

  // Live validation events (only meaningful during active pending_merge)
  const liveSteps = useMergeValidationEvents(task.id);

  // Parse conflict files from task metadata if available
  const conflictFiles: string[] = (() => {
    if (!task.metadata) return [];
    const metadata = typeof task.metadata === "string"
      ? JSON.parse(task.metadata)
      : task.metadata;
    return Array.isArray(metadata?.conflict_files) ? metadata.conflict_files : [];
  })();

  const branchName = task.taskBranch ?? "task branch";

  // Determine labels/icons based on validation recovery vs conflict resolution
  const isRecoveryAgent = isAgentPhase && isValidationRecovery;

  const statusLabel = historicalOutcome
    ? historicalOutcome === "merged"
      ? "Merged"
      : "Conflict"
    : isHistorical
    ? isProgrammaticPhase
      ? "Attempted"
      : isValidationRecovery ? "Fixing" : "Resolving"
    : isProgrammaticPhase
    ? "Merging"
    : isValidationRecovery ? "Fixing" : "Resolving";
  const statusIcon = historicalOutcome
    ? historicalOutcome === "merged"
      ? CheckCircle2
      : AlertTriangle
    : isHistorical
    ? isProgrammaticPhase
      ? GitMerge
      : isValidationRecovery ? Wrench : AlertTriangle
    : isProgrammaticPhase
    ? GitMerge
    : isValidationRecovery ? Wrench : AlertTriangle;

  return (
    <TwoColumnLayout
      description={task.description}
      testId="merging-task-detail"
    >
      {/* Status Banner */}
      <StatusBanner
        icon={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? CheckCircle2
              : AlertTriangle
            : isHistorical
            ? isProgrammaticPhase
              ? GitMerge
              : isValidationRecovery ? Wrench : AlertTriangle
            : isProgrammaticPhase
            ? GitMerge
            : isValidationRecovery ? Wrench : Bot
        }
        title={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "Merge Completed"
              : "Merge Conflict"
            : isHistorical
            ? isProgrammaticPhase
              ? "Merge Attempted"
              : isValidationRecovery ? "Fixing Validation Errors" : "Resolving Conflicts"
            : isProgrammaticPhase
            ? "Merging Changes..."
            : isValidationRecovery ? "Fixing Validation Errors..." : "Resolving Merge Conflicts"
        }
        subtitle={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "Final outcome: merged into base branch"
              : "Final outcome: manual resolution required"
            : isHistorical
            ? isProgrammaticPhase
              ? "Merge attempt captured in history"
              : isValidationRecovery ? "Agent was fixing build errors" : "Agent was resolving conflicts"
            : isProgrammaticPhase
            ? `Attempting to merge ${branchName}`
            : isValidationRecovery ? "AI agent is fixing build errors" : "AI agent is resolving conflicts"
        }
        variant={
          historicalOutcome
            ? historicalOutcome === "merged"
              ? "success"
              : "warning"
            : isHistorical
            ? isProgrammaticPhase
              ? "info"
              : isValidationRecovery ? "accent" : "warning"
            : isProgrammaticPhase
            ? "accent"
            : isValidationRecovery ? "accent" : "warning"
        }
        animated={!isHistorical}
        badge={
          <StatusPill
            icon={statusIcon}
            label={statusLabel}
            variant={
              historicalOutcome
                ? historicalOutcome === "merged"
                  ? "success"
                  : "warning"
                : isHistorical
                ? isProgrammaticPhase
                  ? "info"
                  : isValidationRecovery ? "accent" : "warning"
                : isProgrammaticPhase
                ? "accent"
                : isValidationRecovery ? "accent" : "warning"
            }
            animated={!isHistorical}
            size="md"
          />
        }
      />

      {/* Merge Progress */}
      <section data-testid="merge-progress-section">
        <SectionTitle>{isHistorical ? "Process Details" : "Merge Progress"}</SectionTitle>
        <DetailCard variant="default">
          <p className="text-[12px] text-white/50 mb-3">
            {historicalOutcome
              ? "Process context captured during the merge lifecycle."
              : isHistorical
              ? historicalMode === "attempted"
                ? "Programmatic merge attempt captured in history."
                : isValidationRecovery
                ? "Agent was fixing validation errors at this point."
                : "Agent was resolving conflicts at this point."
              : isProgrammaticPhase
              ? "Programmatic merge attempt in progress."
              : isValidationRecovery
              ? "Agent is fixing validation errors; falls back to manual if unsuccessful."
              : "Agent is resolving conflicts; manual resolution may be required."}
          </p>
          <MergeProgressSteps
            isProgrammaticPhase={isProgrammaticPhase}
            isHistorical={isHistorical}
            historicalMode={historicalMode}
            isValidationRecovery={isValidationRecovery}
          />
        </DetailCard>
      </section>

      {/* Validation Failures (only in recovery mode during agent phase) */}
      {isRecoveryAgent && validationFailures.length > 0 && (
        <section data-testid="validation-failures-section">
          <SectionTitle>Validation Failures ({validationFailures.length})</SectionTitle>
          <DetailCard variant="warning">
            <ValidationFailuresList failures={validationFailures} />
          </DetailCard>
        </section>
      )}

      {/* Validation Progress (live or historical) */}
      <ValidationProgress
        taskId={task.id}
        metadata={task.metadata}
        liveSteps={liveSteps}
      />

      {/* Conflict Files (only for agent phase, non-recovery) */}
      {isAgentPhase && !isValidationRecovery && conflictFiles.length > 0 && (
        <section data-testid="conflict-files-section">
          <SectionTitle>Conflict Files ({conflictFiles.length})</SectionTitle>
          <DetailCard variant="warning">
            <ConflictFilesList files={conflictFiles} />
          </DetailCard>
        </section>
      )}

      {/* Branch Info */}
      <section data-testid="branch-info-section">
        <SectionTitle muted>Branch</SectionTitle>
        <p className="text-[12px] text-white/50 font-mono">
          {branchName}
        </p>
      </section>
    </TwoColumnLayout>
  );
}
