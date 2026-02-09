/**
 * ContextWidget — Always-visible context loaded card for get_task_context
 *
 * Design reference: mockups/tool-call-widgets.html (Widget 1)
 * NOT collapsible — compact summary with orange left accent border
 */

import React from "react";
import { Badge } from "./shared";
import { colors } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";

// ============================================================================
// Raw result types (snake_case from MCP/CLI JSON)
// ============================================================================

interface RawTask {
  title?: string;
  category?: string;
  priority?: number;
}

interface RawPlanArtifact {
  title?: string;
}

interface RawStep {
  id?: string;
  status?: string;
}

interface RawTaskContext {
  task?: RawTask;
  plan_artifact?: RawPlanArtifact;
  steps?: RawStep[];
  step_progress?: { total?: number };
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
  if (ctx.step_progress?.total != null) return ctx.step_progress.total;
  if (ctx.steps) return ctx.steps.length;
  return 0;
}

// ============================================================================
// ContextWidget
// ============================================================================

export const ContextWidget = React.memo(function ContextWidget({
  toolCall,
  compact = false,
}: ToolCallWidgetProps) {
  const ctx = toolCall.result as RawTaskContext | undefined;
  if (!ctx?.task) return null;

  const { task, plan_artifact } = ctx;
  const stepCount = getStepCount(ctx);
  const priority = task.priority != null ? getPriorityLabel(task.priority) : null;

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
        {stepCount > 0 && (
          <Badge variant="muted" compact>{stepCount} step{stepCount !== 1 ? "s" : ""}</Badge>
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
    </div>
  );
});
