import { formatDuration } from "./tool-call-utils";
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
