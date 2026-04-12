import React from "react";
import {
  formatMessageAttributionTooltip,
  formatProviderHarnessLabel,
  formatProviderModelEffortLabel,
  getProviderHarnessBadgeStyle,
  type ProviderMetadata,
} from "./provider-harness";
import { formatDuration, getModelColor, getSubagentTypeColor } from "./tool-call-utils";
import { canonicalizeToolName } from "./tool-widgets/tool-name";

export interface TaskCardSummaryMetrics {
  totalDurationMs?: number | null | undefined;
  totalTokens?: number | null | undefined;
  totalToolUseCount?: number | null | undefined;
  estimatedUsd?: number | null | undefined;
}

export function getTaskCardKindLabel(toolName: string): "Delegate" | "Agent" | "Task" {
  const canonical = canonicalizeToolName(toolName);
  if (canonical === "delegate_start") return "Delegate";
  if (canonical === "agent") return "Agent";
  return "Task";
}

export function buildTaskCardSummaryParts({
  totalDurationMs,
  totalTokens,
  totalToolUseCount,
  estimatedUsd,
}: TaskCardSummaryMetrics): string[] {
  const parts: string[] = [];

  if (totalDurationMs != null) {
    parts.push(formatDuration(totalDurationMs));
  }
  if (totalTokens != null) {
    parts.push(`${totalTokens.toLocaleString()} tokens`);
  }
  if (totalToolUseCount != null) {
    parts.push(`${totalToolUseCount} tool${totalToolUseCount !== 1 ? "s" : ""}`);
  }
  if (estimatedUsd != null) {
    parts.push(`$${estimatedUsd.toFixed(2)}`);
  }

  return parts;
}

function TaskCardBadge({
  children,
  className = "",
  style,
  title,
}: {
  children: React.ReactNode;
  className?: string | undefined;
  style: React.CSSProperties;
  title?: string | undefined;
}) {
  return (
    <span
      className={`text-[10px] px-1.5 py-0.5 rounded flex-shrink-0 ${className}`.trim()}
      style={style}
      title={title}
    >
      {children}
    </span>
  );
}

export function TaskCardKindBadge({ toolName }: { toolName: string }) {
  const kind = getTaskCardKindLabel(toolName);

  const style =
    kind === "Delegate"
      ? {
          backgroundColor: "hsla(150, 55%, 45%, 0.12)",
          color: "hsl(150, 55%, 63%)",
        }
      : kind === "Agent"
        ? {
            backgroundColor: "hsla(14, 100%, 60%, 0.12)",
            color: "hsl(14, 100%, 65%)",
          }
        : {
            backgroundColor: "hsla(220, 10%, 50%, 0.12)",
            color: "hsl(220, 10%, 60%)",
          };

  return (
    <TaskCardBadge className="font-medium" style={style}>
      {kind}
    </TaskCardBadge>
  );
}

export function TaskCardSubagentTypeBadge({
  subagentType,
}: {
  subagentType?: string | null | undefined;
}) {
  if (!subagentType || subagentType === "agent") {
    return null;
  }

  const color = getSubagentTypeColor(subagentType);
  return (
    <TaskCardBadge
      className="font-medium"
      style={{
        backgroundColor: color.bg,
        color: color.text,
      }}
    >
      {subagentType}
    </TaskCardBadge>
  );
}

export function TaskCardProviderHarnessBadge({
  providerHarness,
  providerMetadata,
}: {
  providerHarness?: string | null | undefined;
  providerMetadata: ProviderMetadata;
}) {
  const label = formatProviderHarnessLabel(providerHarness);
  if (!label) {
    return null;
  }

  return (
    <TaskCardBadge
      className="font-medium"
      style={getProviderHarnessBadgeStyle(providerHarness)}
      title={formatMessageAttributionTooltip(providerMetadata) ?? undefined}
    >
      {label}
    </TaskCardBadge>
  );
}

export function TaskCardModelBadge({
  label,
  colorKey,
  providerMetadata,
}: {
  label?: string | null | undefined;
  colorKey?: string | null | undefined;
  providerMetadata?: ProviderMetadata | undefined;
}) {
  if (!label) {
    return null;
  }

  const color = getModelColor(colorKey ?? label);
  const tooltip = providerMetadata
    ? formatMessageAttributionTooltip(providerMetadata) ?? undefined
    : undefined;

  return (
    <TaskCardBadge
      style={{
        backgroundColor: color.bg,
        color: color.text,
      }}
      title={tooltip}
    >
      {label}
    </TaskCardBadge>
  );
}

export function TaskCardProviderModelBadge({
  providerMetadata,
}: {
  providerMetadata: ProviderMetadata;
}) {
  const label = formatProviderModelEffortLabel(providerMetadata);
  const colorKey = providerMetadata.effectiveModelId ?? providerMetadata.logicalModel ?? label;
  return (
    <TaskCardModelBadge
      label={label}
      colorKey={colorKey}
      providerMetadata={providerMetadata}
    />
  );
}

export function TaskCardStatusBadge({
  label,
  tone = "warning",
}: {
  label?: string | null | undefined;
  tone?: "warning" | "error";
}) {
  if (!label) {
    return null;
  }

  const style =
    tone === "error"
      ? {
          backgroundColor: "hsla(0 70% 50% / 0.18)",
          color: "hsl(0 70% 70%)",
        }
      : {
          backgroundColor: "hsla(38 90% 50% / 0.15)",
          color: "hsl(38 90% 60%)",
        };

  return (
    <TaskCardBadge className="font-medium" style={style}>
      {label}
    </TaskCardBadge>
  );
}

export function TaskCardSummary({
  metrics,
  className = "",
}: {
  metrics: TaskCardSummaryMetrics;
  className?: string;
}) {
  const parts = buildTaskCardSummaryParts(metrics);
  if (parts.length === 0) {
    return null;
  }

  return (
    <span
      className={`text-xs ${className}`.trim()}
      style={{ color: "var(--text-muted, hsl(220 10% 50%))" }}
    >
      {parts.join(" \u00B7 ")}
    </span>
  );
}
