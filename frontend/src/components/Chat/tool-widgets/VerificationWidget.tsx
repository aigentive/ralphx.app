/**
 * VerificationWidget — Compact indicators for plan verification and child session status tools
 *
 * Handles:
 * - update_plan_verification: verification state (round, gaps, convergence)
 * - get_plan_verification: verification state reader
 * - get_child_session_status: child session status check
 * - get_verification_confirmation_status: confirmation status for a session
 * - get_pending_confirmations: pending confirmations count
 */

import React from "react";
import { ShieldCheck, GitBranch, Bell } from "lucide-react";
import { InlineIndicator, Badge, WidgetRow } from "./shared";
import {
  colors,
  getString,
  getNumber,
  getArray,
  getBool,
  parseMcpToolResult,
  truncatedTitleStyle,
  truncate,
  badgeStyles,
} from "./shared.constants";
import type { ToolCallWidgetProps, BadgeVariant } from "./shared.constants";

// ============================================================================
// Helpers
// ============================================================================

type VerificationTool =
  | "update_plan_verification"
  | "get_plan_verification"
  | "get_child_session_status"
  | "get_verification_confirmation_status"
  | "get_pending_confirmations";

function getToolType(toolName: string): VerificationTool | null {
  const name = toolName.toLowerCase();
  if (name.includes("update_plan_verification")) return "update_plan_verification";
  if (name.includes("get_plan_verification")) return "get_plan_verification";
  if (name.includes("get_child_session_status")) return "get_child_session_status";
  if (name.includes("get_verification_confirmation_status")) return "get_verification_confirmation_status";
  if (name.includes("get_pending_confirmations")) return "get_pending_confirmations";
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

/** Continuity data from verification_child block (camelCase fields per VerificationChildInfo serde). */
interface VerificationChildData {
  latestChildSessionId?: string;
  agentState?: string;
  lastAssistantMessage?: string | null;
}

function GetVerification({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const status = getString(parsed, "status");
  const inProgress = getBool(parsed, "in_progress");
  const currentRound = getNumber(parsed, "current_round");
  const maxRounds = getNumber(parsed, "max_rounds");
  const convergenceReason = getString(parsed, "convergence_reason");
  // verification_child uses snake_case field name; inner fields are camelCase per VerificationChildInfo serde
  const verificationChild = (parsed["verification_child"] ?? null) as VerificationChildData | null;

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

  const mainRow = (
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

  if (verificationChild != null) {
    const sessionId = typeof verificationChild.latestChildSessionId === "string"
      ? verificationChild.latestChildSessionId
      : undefined;
    const agentState = typeof verificationChild.agentState === "string"
      ? verificationChild.agentState
      : undefined;
    const lastMessage = typeof verificationChild.lastAssistantMessage === "string"
      ? verificationChild.lastAssistantMessage
      : undefined;

    const agentInfo = agentStatusLabel(agentState);
    const sessionSnippet = sessionId ? sessionId.slice(0, 8) : undefined;
    const messagePreview = lastMessage ? truncate(lastMessage, 120) : undefined;

    return (
      <>
        {mainRow}
        <WidgetRow compact={compact}>
          <GitBranch size={11} style={{ color: colors.textMuted, flexShrink: 0 }} />
          {sessionSnippet !== undefined && (
            <span style={{ fontSize: compact ? 10 : 10.5, color: colors.textMuted, fontFamily: "monospace" }}>
              {sessionSnippet}…
            </span>
          )}
          <Badge variant={agentInfo.variant} compact>{agentInfo.label}</Badge>
          {messagePreview !== undefined && (
            <span style={{ fontSize: compact ? 10 : 10.5, color: colors.textMuted, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>
              {messagePreview}
            </span>
          )}
        </WidgetRow>
      </>
    );
  }

  return mainRow;
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

function VerificationConfirmationStatus({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const status = getString(parsed, "status");

  if (!status) {
    return (
      <InlineIndicator
        icon={<ShieldCheck size={11} style={{ color: colors.textMuted }} />}
        text="Checking confirmation status..."
      />
    );
  }

  const variant: BadgeVariant = status === "pending" ? "accent" : "muted";
  const label = status === "pending" ? "Pending" : status === "not_applicable" ? "N/A" : status;
  const iconColor = badgeStyles[variant].color;

  return (
    <WidgetRow compact={compact}>
      <ShieldCheck size={12} style={{ color: iconColor, flexShrink: 0 }} />
      <Badge variant={variant} compact>{label}</Badge>
    </WidgetRow>
  );
}

function PendingConfirmations({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const sessions = getArray(parsed, "sessions");

  if (!sessions) {
    return (
      <InlineIndicator
        icon={<Bell size={11} style={{ color: colors.textMuted }} />}
        text="Checking pending confirmations..."
      />
    );
  }

  const count = sessions.length;
  const variant: BadgeVariant = count > 0 ? "accent" : "muted";
  const label = count > 0 ? `${count} pending` : "No pending";
  const iconColor = badgeStyles[variant].color;

  return (
    <WidgetRow compact={compact}>
      <Bell size={12} style={{ color: iconColor, flexShrink: 0 }} />
      <Badge variant={variant} compact>{label}</Badge>
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
      return <div data-testid="verification-widget-update"><UpdateVerification {...props} /></div>;
    case "get_plan_verification":
      return <div data-testid="verification-widget-get"><GetVerification {...props} /></div>;
    case "get_child_session_status":
      return <div data-testid="verification-widget-child-status"><ChildSessionStatus {...props} /></div>;
    case "get_verification_confirmation_status":
      return <div data-testid="verification-widget-confirmation"><VerificationConfirmationStatus {...props} /></div>;
    case "get_pending_confirmations":
      return <div data-testid="verification-widget-pending"><PendingConfirmations {...props} /></div>;
    default:
      return <div data-testid="verification-widget-fallback"><InlineIndicator text={props.toolCall.name} /></div>;
  }
});
