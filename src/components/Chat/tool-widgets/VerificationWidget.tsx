/**
 * VerificationWidget — Compact indicators for plan verification and child session status tools
 *
 * Handles:
 * - update_plan_verification: verification state (round, gaps, convergence)
 * - get_plan_verification: verification state reader
 * - get_child_session_status: child session status check
 */

import React from "react";
import { ShieldCheck, GitBranch } from "lucide-react";
import { InlineIndicator, Badge, WidgetRow } from "./shared";
import {
  colors,
  getString,
  getNumber,
  getArray,
  getBool,
  parseMcpToolResult,
  truncatedTitleStyle,
} from "./shared.constants";
import type { ToolCallWidgetProps, BadgeVariant } from "./shared.constants";

// ============================================================================
// Helpers
// ============================================================================

type VerificationTool =
  | "update_plan_verification"
  | "get_plan_verification"
  | "get_child_session_status";

function getToolType(toolName: string): VerificationTool | null {
  const name = toolName.toLowerCase();
  if (name.includes("update_plan_verification")) return "update_plan_verification";
  if (name.includes("get_plan_verification")) return "get_plan_verification";
  if (name.includes("get_child_session_status")) return "get_child_session_status";
  return null;
}

function convergenceLabel(reason: string | undefined): string | undefined {
  if (!reason) return undefined;
  const map: Record<string, string> = {
    zero_blocking: "All gaps resolved",
    jaccard_converged: "Gaps converged",
    max_rounds: "Max rounds",
    critic_parse_failure: "Parse failure",
    agent_error: "Agent error",
    user_stopped: "Stopped",
    user_skipped: "Skipped",
    user_reverted: "Reverted",
    escalated_to_parent: "Escalated",
  };
  return map[reason] ?? reason;
}

function statusBadgeVariant(status: string | undefined): BadgeVariant {
  switch (status) {
    case "reviewing": return "blue";
    case "needs_revision": return "accent";
    case "verified": return "success";
    case "imported_verified": return "success";
    case "skipped": return "muted";
    case "unverified": return "muted";
    default: return "muted";
  }
}

function iconColorForVariant(variant: BadgeVariant): string {
  switch (variant) {
    case "success": return colors.success;
    case "blue": return colors.blue;
    case "accent": return colors.accent;
    case "error": return colors.error;
    default: return colors.textMuted;
  }
}

function agentStatusLabel(status: string | undefined): { label: string; variant: BadgeVariant } {
  switch (status) {
    case "likely_generating": return { label: "Generating", variant: "blue" };
    case "likely_waiting": return { label: "Waiting", variant: "accent" };
    case "idle": return { label: "Idle", variant: "muted" };
    default: return { label: status ?? "Unknown", variant: "muted" };
  }
}

// ============================================================================
// Sub-renderers
// ============================================================================

function UpdateVerification({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const status = getString(parsed, "status");
  const currentRound = getNumber(parsed, "current_round");
  const maxRounds = getNumber(parsed, "max_rounds");
  const gaps = getArray(parsed, "current_gaps");
  const convergenceReason = getString(parsed, "convergence_reason");

  if (!status) {
    return (
      <InlineIndicator
        icon={<ShieldCheck size={11} style={{ color: colors.textMuted }} />}
        text="Updating verification..."
      />
    );
  }

  const variant = statusBadgeVariant(status);
  const iconColor = iconColorForVariant(variant);
  const gapCount = gaps?.length ?? 0;
  const hasHighSeverity = gaps?.some((g) => {
    const severity = getString(g as Record<string, unknown>, "severity");
    return severity === "critical" || severity === "high";
  });
  const convLabel = convergenceLabel(convergenceReason);

  return (
    <WidgetRow compact={compact}>
      <ShieldCheck size={12} style={{ color: iconColor, flexShrink: 0 }} />
      {currentRound != null && maxRounds != null && (
        <span style={{ fontSize: compact ? 10.5 : 11, color: colors.textSecondary }}>
          Round {currentRound}/{maxRounds}
        </span>
      )}
      <Badge variant={variant} compact>{status}</Badge>
      {gapCount > 0 && (
        <Badge variant={hasHighSeverity ? "error" : "accent"} compact>{gapCount} gaps</Badge>
      )}
      {convLabel && <Badge variant="muted" compact>{convLabel}</Badge>}
    </WidgetRow>
  );
}

function GetVerification({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const status = getString(parsed, "status");
  const inProgress = getBool(parsed, "in_progress");
  const currentRound = getNumber(parsed, "current_round");
  const maxRounds = getNumber(parsed, "max_rounds");
  const convergenceReason = getString(parsed, "convergence_reason");

  if (!status) {
    return (
      <InlineIndicator
        icon={<ShieldCheck size={11} style={{ color: colors.textMuted }} />}
        text="Loading verification..."
      />
    );
  }

  const variant = statusBadgeVariant(status);
  const iconColor = iconColorForVariant(variant);
  const convLabel = convergenceLabel(convergenceReason);
  const showRound = (inProgress === true || currentRound != null) && currentRound != null && maxRounds != null;
  const showConvergence = (status === "verified" || status === "skipped" || status === "imported_verified") && convLabel;

  return (
    <WidgetRow compact={compact}>
      <ShieldCheck size={12} style={{ color: iconColor, flexShrink: 0 }} />
      <Badge variant={variant} compact>{status}</Badge>
      {showRound && (
        <span style={{ fontSize: compact ? 10.5 : 11, color: colors.textSecondary }}>
          Round {currentRound}/{maxRounds}
        </span>
      )}
      {showConvergence && <Badge variant="muted" compact>{convLabel}</Badge>}
    </WidgetRow>
  );
}

function ChildSessionStatus({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const session = parsed.session as Record<string, unknown> | undefined;
  const agentState = parsed.agent_state as Record<string, unknown> | undefined;
  const verification = parsed.verification as Record<string, unknown> | undefined;

  const title = session ? getString(session, "title") : undefined;
  const estimatedStatus = agentState ? getString(agentState, "estimated_status") : undefined;
  const verificationRound = verification ? getNumber(verification, "current_round") : undefined;

  if (!title && !estimatedStatus) {
    return (
      <InlineIndicator
        icon={<GitBranch size={11} style={{ color: colors.blue }} />}
        text="Loading session status..."
      />
    );
  }

  const agentInfo = agentStatusLabel(estimatedStatus);

  return (
    <WidgetRow compact={compact}>
      <GitBranch size={12} style={{ color: colors.blue, flexShrink: 0 }} />
      {title && (
        <span style={truncatedTitleStyle(compact)}>{title}</span>
      )}
      <Badge variant={agentInfo.variant} compact>{agentInfo.label}</Badge>
      {verificationRound != null && (
        <Badge variant="blue" compact>Round {verificationRound}</Badge>
      )}
    </WidgetRow>
  );
}

// ============================================================================
// VerificationWidget (main component)
// ============================================================================

export const VerificationWidget = React.memo(function VerificationWidget(props: ToolCallWidgetProps) {
  const toolType = getToolType(props.toolCall.name);

  switch (toolType) {
    case "update_plan_verification":
      return <UpdateVerification {...props} />;
    case "get_plan_verification":
      return <GetVerification {...props} />;
    case "get_child_session_status":
      return <ChildSessionStatus {...props} />;
    default:
      return <InlineIndicator text={props.toolCall.name} />;
  }
});
