/**
 * ContextWidget — Always-visible context loaded card for get_task_context
 *
 * Design reference: mockups/tool-call-widgets.html (Widget 1)
 * NOT collapsible — compact summary with orange left accent border
 */

import React, { useMemo } from "react";
import { Badge, InlineIndicator } from "./shared";
import { colors, parseMcpToolResult, getArray } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";

// ============================================================================
// Raw result types (snake_case from MCP/CLI JSON)
// ============================================================================

interface RawTask {
  title?: string;
  category?: string;
  priority?: number;
  internal_status?: string;
}

interface RawPlanArtifact {
  title?: string;
}

interface RawArtifactSummary {
  id?: string;
  title?: string;
  artifact_type?: string;
}

interface RawTaskContext {
  task?: RawTask;
  plan_artifact?: RawPlanArtifact;
  related_artifacts?: RawArtifactSummary[];
  steps?: unknown[];
  step_progress?: { total_steps?: number; completed_steps?: number };
}

// ============================================================================
// Helpers
// ============================================================================

function getPriorityLabel(priority: number): { label: string; variant: "error" | "accent" | "muted" } {
  if (priority >= 80) return { label: "critical", variant: "error" };
  if (priority >= 60) return { label: "high", variant: "error" };
  if (priority >= 40) return { label: "medium", variant: "accent" };
  return { label: "low", variant: "muted" };
}

function getStepCount(ctx: RawTaskContext): number {
  if (ctx.step_progress?.total_steps != null) return ctx.step_progress.total_steps;
  if (ctx.steps) return ctx.steps.length;
  return 0;
}

function getStatusVariant(status: string): "accent" | "success" | "error" | "muted" {
  switch (status) {
    case "executing":
    case "re_executing":
      return "accent";
    case "approved":
    case "merged":
      return "success";
    case "failed":
    case "cancelled":
      return "error";
    default:
      return "muted";
  }
}

// ============================================================================
// ContextWidget
// ============================================================================

export const ContextWidget = React.memo(function ContextWidget({
  toolCall,
  compact = false,
}: ToolCallWidgetProps) {
  // useMemo must be called unconditionally (React hooks rules)
  const parsed = useMemo(() => parseMcpToolResult(toolCall.result), [toolCall.result]);
  const ctx = parsed as RawTaskContext;

  // Loading state — checked after hook call but before accessing parsed data
  // parseMcpToolResult(null) returns {} so we must check toolCall.result directly
  if (!toolCall.result) {
    return <InlineIndicator text="Loading context..." />;
  }

  if (!ctx?.task) return null;

  const { task, plan_artifact } = ctx;
  const stepCount = getStepCount(ctx);
  const completedSteps = ctx.step_progress?.completed_steps;
  const totalSteps = ctx.step_progress?.total_steps;
  const priority = task.priority != null ? getPriorityLabel(task.priority) : null;
  const status = task.internal_status;
  const relatedArtifacts = getArray(parsed, "related_artifacts") as RawArtifactSummary[] | undefined;
  const visibleArtifacts = relatedArtifacts?.slice(0, 3) ?? [];
  const extraArtifacts = (relatedArtifacts?.length ?? 0) - visibleArtifacts.length;

  const stepLabel = completedSteps != null && totalSteps != null
    ? `${completedSteps}/${totalSteps} step${totalSteps !== 1 ? "s" : ""}`
    : stepCount > 0
      ? `${stepCount} step${stepCount !== 1 ? "s" : ""}`
      : null;

  return (
    <div
      style={{
        background: colors.bgSurface,
        borderRadius: 10,
        border: `1px solid ${colors.borderSubtle}`,
        borderLeft: `2px solid ${colors.accent}`,
        padding: compact ? "6px 8px" : "8px 10px",
      }}
    >
      {/* Top row: spacer + CONTEXT LOADED label */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          justifyContent: "space-between",
        }}
      >
        <div />
        <span
          style={{
            fontSize: 10,
            color: colors.textMuted,
            fontWeight: 500,
            letterSpacing: "0.02em",
            textTransform: "uppercase" as const,
          }}
        >
          Context loaded
        </span>
      </div>

      {/* Task title */}
      <div
        style={{
          fontSize: compact ? 11 : 12,
          fontWeight: 600,
          color: colors.textPrimary,
          marginTop: 4,
          lineHeight: 1.35,
        }}
      >
        {task.title || "Untitled task"}
      </div>

      {/* Chips row */}
      <div
        style={{
          display: "flex",
          gap: 5,
          marginTop: 6,
          flexWrap: "wrap" as const,
        }}
      >
        {task.category && (
          <Badge variant="accent" compact>{task.category}</Badge>
        )}
        {priority && (
          <Badge variant={priority.variant} compact>{priority.label}</Badge>
        )}
        {status && (
          <Badge variant={getStatusVariant(status)} compact>● {status.replace(/_/g, " ")}</Badge>
        )}
        {stepLabel && (
          <Badge variant="muted" compact>{stepLabel}</Badge>
        )}
      </div>

      {/* Source plan artifact name */}
      {plan_artifact?.title && (
        <div
          style={{
            fontSize: 10.5,
            color: colors.textMuted,
            marginTop: 5,
          }}
        >
          Source: {plan_artifact.title}
        </div>
      )}

      {/* Related artifact references (capped at 3) */}
      {visibleArtifacts.length > 0 && (
        <div style={{ marginTop: 6, display: "flex", flexDirection: "column", gap: 3 }}>
          {visibleArtifacts.map((artifact, idx) => (
            <div key={idx} style={{ display: "flex", alignItems: "center", gap: 5 }}>
              {artifact.artifact_type && (
                <span
                  style={{
                    fontSize: 9,
                    padding: "1px 4px",
                    borderRadius: 3,
                    fontWeight: 600,
                    textTransform: "uppercase" as const,
                    letterSpacing: "0.04em",
                    background: "var(--bg-hover)",
                    color: "var(--text-muted)",
                    flexShrink: 0,
                  }}
                >
                  {artifact.artifact_type.slice(0, 8).toUpperCase()}
                </span>
              )}
              <span
                style={{
                  fontSize: 10,
                  color: colors.textSecondary,
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap" as const,
                }}
              >
                {artifact.title ?? "Untitled"}
              </span>
            </div>
          ))}
          {extraArtifacts > 0 && (
            <div style={{ fontSize: 9.5, color: colors.textMuted }}>
              +{extraArtifacts} more
            </div>
          )}
        </div>
      )}
    </div>
  );
});
