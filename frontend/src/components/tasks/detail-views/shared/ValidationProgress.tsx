/**
 * ValidationProgress - Shared components for displaying validation step progress
 *
 * Used by MergingTaskDetail, MergedTaskDetail, and MergeIncompleteTaskDetail
 * to show real-time or historical validation command execution.
 */

import { useState, useMemo } from "react";
import {
  Loader2,
  XCircle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock,
  Archive,
  SkipForward,
} from "lucide-react";
import { SectionTitle } from "./SectionTitle";
import type { MergeValidationStepEvent } from "@/types/events";

function formatDuration(ms: number): string {
  return ms < 1000 ? `${ms}ms` : `${(ms / 1000).toFixed(1)}s`;
}

/**
 * ValidationStepRow - Single validation command row with collapsible output
 */
export function ValidationStepRow({ step }: { step: MergeValidationStepEvent }) {
  const isRunning = step.status === "running";
  const isFailed = step.status === "failed";
  const isCached = step.status === "cached";
  const isSkipped = step.status === "skipped";
  const hasOutput = (step.stdout && step.stdout.trim().length > 0) ||
    (step.stderr && step.stderr.trim().length > 0);
  const [expanded, setExpanded] = useState(isRunning || isFailed);

  const statusIcon = isRunning ? (
    <Loader2 className="w-4 h-4 animate-spin" style={{ color: "var(--status-info)" }} />
  ) : isFailed ? (
    <XCircle className="w-4 h-4" style={{ color: "#ff453a" }} />
  ) : isSkipped ? (
    <SkipForward className="w-4 h-4" style={{ color: "rgba(255,255,255,0.3)" }} />
  ) : isCached ? (
    <Archive className="w-4 h-4" style={{ color: "var(--status-success)" }} />
  ) : (
    <CheckCircle2 className="w-4 h-4" style={{ color: "var(--status-success)" }} />
  );

  return (
    <div
      className="rounded-lg overflow-hidden"
      style={{ backgroundColor: "var(--overlay-scrim)" }}
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
          style={{ backgroundColor: "var(--accent-muted)", color: "var(--accent-primary)" }}
        >
          validate
        </span>
        <span className="text-[12px] text-white/80 font-mono truncate flex-1" title={step.label}>
          {step.label}
        </span>
        {isCached && (
          <span
            className="text-[9px] uppercase font-semibold tracking-wider px-1.5 py-0.5 rounded shrink-0"
            style={{ backgroundColor: "var(--status-success-muted)", color: "var(--status-success)" }}
          >
            Cached
          </span>
        )}
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
 * StepsGroup - Collapsible group of steps with summary header.
 * Used for setup steps and passed validation checks.
 */
export function StepsGroup({ steps, phase, label }: {
  steps: MergeValidationStepEvent[];
  phase: string;
  label: string;
}) {
  const anyFailed = steps.some((s) => s.status === "failed");
  const anyRunning = steps.some((s) => s.status === "running");
  const [expanded, setExpanded] = useState(anyFailed);

  const totalMs = steps.reduce((sum, s) => sum + (s.duration_ms ?? 0), 0);
  const badgeBg = phase === "setup" ? "var(--status-info-muted)" : phase === "install" ? "var(--accent-muted)" : phase === "skipped" ? "rgba(255, 255, 255, 0.08)" : "var(--status-success-muted)";
  const badgeColor = phase === "setup" ? "var(--status-info)" : phase === "install" ? "var(--accent-primary)" : phase === "skipped" ? "rgba(255, 255, 255, 0.4)" : "var(--status-success)";

  const allSkipped = steps.every((s) => s.status === "skipped");
  const statusIcon = anyRunning ? (
    <Loader2 className="w-4 h-4 animate-spin" style={{ color: "var(--status-info)" }} />
  ) : anyFailed ? (
    <XCircle className="w-4 h-4" style={{ color: "#ff453a" }} />
  ) : allSkipped ? (
    <SkipForward className="w-4 h-4" style={{ color: "rgba(255,255,255,0.3)" }} />
  ) : (
    <CheckCircle2 className="w-4 h-4" style={{ color: "var(--status-success)" }} />
  );

  return (
    <div
      className="rounded-lg overflow-hidden"
      style={{ backgroundColor: "var(--overlay-scrim)" }}
    >
      <button
        type="button"
        className="w-full flex items-center gap-2.5 px-3 py-2.5 text-left cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        {statusIcon}
        <span
          className="text-[10px] uppercase font-semibold tracking-wider px-1.5 py-0.5 rounded"
          style={{ backgroundColor: badgeBg, color: badgeColor }}
        >
          {phase}
        </span>
        <span className="text-[12px] text-white/80 flex-1">
          {label}
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
            const isSkipped = step.status === "skipped";
            const hasOutput = (step.stdout && step.stdout.trim().length > 0) ||
              (step.stderr && step.stderr.trim().length > 0);
            return (
              <div key={`${phase}-${step.command}-${i}`} className="space-y-1">
                <div className="flex items-center gap-2">
                  {isRunning ? (
                    <Loader2 className="w-3 h-3 animate-spin" style={{ color: "var(--status-info)" }} />
                  ) : isFailed ? (
                    <XCircle className="w-3 h-3" style={{ color: "#ff453a" }} />
                  ) : isSkipped ? (
                    <SkipForward className="w-3 h-3" style={{ color: "rgba(255,255,255,0.3)" }} />
                  ) : (
                    <CheckCircle2 className="w-3 h-3" style={{ color: "var(--status-success)" }} />
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
function parseMetadataValidationLog(
  metadata: string | Record<string, unknown> | null | undefined,
  logKey = "validation_log",
): MergeValidationStepEvent[] {
  if (!metadata) return [];
  try {
    const parsed = typeof metadata === "string" ? JSON.parse(metadata) : metadata;
    const log = parsed?.[logKey];
    if (!Array.isArray(log)) return [];
    return log.map((entry: Record<string, unknown>) => ({
      task_id: String(entry.task_id ?? ""),
      phase: (entry.phase === "setup" || entry.phase === "validate" || entry.phase === "install") ? entry.phase : "validate",
      command: String(entry.command ?? ""),
      path: String(entry.path ?? ""),
      label: String(entry.label ?? entry.command ?? ""),
      status: (entry.status === "running" || entry.status === "success" || entry.status === "failed" || entry.status === "cached" || entry.status === "skipped")
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
 * - Live: useValidationEvents(task.id, context) — real-time events during execution/merge
 * - Historical: task.metadata.validation_log — stored after merge attempt
 * - Merge: prefer live events; fall back to metadata if live is empty
 */
export function ValidationProgress({
  taskId,
  metadata,
  liveSteps,
  title = "Merge Validation",
  metadataLogKey = "validation_log",
}: {
  taskId: string;
  metadata?: string | Record<string, unknown> | null | undefined;
  liveSteps?: MergeValidationStepEvent[] | undefined;
  title?: string;
  metadataLogKey?: string;
}) {
  const metadataSteps = useMemo(() => parseMetadataValidationLog(metadata, metadataLogKey), [metadata, metadataLogKey]);
  const steps = liveSteps && liveSteps.length > 0 ? liveSteps : metadataSteps;

  if (steps.length === 0) return null;

  const source = liveSteps && liveSteps.length > 0 ? "live" : "historical";
  const setupSteps = steps.filter((s) => s.phase === "setup");
  const installSteps = steps.filter((s) => s.phase === "install");
  const validateSteps = steps.filter((s) => s.phase !== "setup" && s.phase !== "install");
  const passedValidateSteps = validateSteps.filter((s) => s.status === "success" || s.status === "cached");
  const skippedValidateSteps = validateSteps.filter((s) => s.status === "skipped");
  const activeValidateSteps = validateSteps.filter((s) => s.status !== "success" && s.status !== "cached" && s.status !== "skipped");

  return (
    <section data-testid={`validation-progress-${taskId}`}>
      <SectionTitle>
        {title}
        {source === "live" && (
          <span className="ml-2 text-[10px] font-normal text-white/30">(live)</span>
        )}
      </SectionTitle>
      <div className="space-y-1.5">
        {setupSteps.length > 0 && (
          <StepsGroup
            steps={setupSteps}
            phase="setup"
            label={`${setupSteps.length} command${setupSteps.length !== 1 ? "s" : ""}`}
          />
        )}
        {installSteps.length > 0 && (
          <StepsGroup
            steps={installSteps}
            phase="install"
            label={`${installSteps.length} command${installSteps.length !== 1 ? "s" : ""}`}
          />
        )}
        {passedValidateSteps.length > 0 && (
          <StepsGroup
            steps={passedValidateSteps}
            phase="passed"
            label={`${passedValidateSteps.length} check${passedValidateSteps.length !== 1 ? "s" : ""} passed`}
          />
        )}
        {activeValidateSteps.map((step, index) => (
          <ValidationStepRow key={`${step.phase}-${step.command}-${index}`} step={step} />
        ))}
        {skippedValidateSteps.length > 0 && (
          <StepsGroup
            steps={skippedValidateSteps}
            phase="skipped"
            label={`${skippedValidateSteps.length} check${skippedValidateSteps.length !== 1 ? "s" : ""} skipped`}
          />
        )}
      </div>
    </section>
  );
}
