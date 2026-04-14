/**
 * VerificationWidget — structured cards for verification status and verifier round tools.
 *
 * The verification chat should expose progress/state, not raw MCP payloads.
 */

import React from "react";
import {
  ShieldCheck,
  GitBranch,
  Bell,
  ListChecks,
  ShieldAlert,
  Sparkles,
} from "lucide-react";
import { InlineIndicator, Badge, WidgetRow, WidgetCard, WidgetHeader } from "./shared";
import {
  colors,
  getString,
  getNumber,
  getArray,
  getBool,
  parseMcpToolResult,
  parseMcpToolResultRaw,
  truncatedTitleStyle,
  truncate,
  badgeStyles,
} from "./shared.constants";
import type { ToolCallWidgetProps, BadgeVariant } from "./shared.constants";

type VerificationTool =
  | "run_verification_enrichment"
  | "run_verification_round"
  | "report_verification_round"
  | "complete_plan_verification"
  | "update_plan_verification"
  | "get_plan_verification"
  | "get_child_session_status"
  | "get_verification_confirmation_status"
  | "get_pending_confirmations";

type VerificationChildData = {
  latestChildSessionId?: string;
  agentState?: string;
  lastAssistantMessage?: string | null;
};

function getToolType(toolName: string): VerificationTool | null {
  const name = toolName.toLowerCase();
  if (name.includes("run_verification_enrichment")) return "run_verification_enrichment";
  if (name.includes("run_verification_round")) return "run_verification_round";
  if (name.includes("report_verification_round")) return "report_verification_round";
  if (name.includes("complete_plan_verification")) return "complete_plan_verification";
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
    case "reviewing":
      return "blue";
    case "needs_revision":
      return "accent";
    case "verified":
    case "imported_verified":
      return "success";
    case "infra_failure":
      return "error";
    case "pending":
      return "warning";
    case "skipped":
    case "unverified":
      return "muted";
    default:
      return "muted";
  }
}

function iconColorForVariant(variant: BadgeVariant): string {
  switch (variant) {
    case "success":
      return colors.success;
    case "blue":
      return colors.blue;
    case "accent":
      return colors.accent;
    case "error":
      return colors.error;
    case "warning":
      return badgeStyles.warning.color;
    default:
      return colors.textMuted;
  }
}

function agentStatusLabel(status: string | undefined): { label: string; variant: BadgeVariant } {
  switch (status) {
    case "running":
    case "queued":
    case "likely_generating":
      return { label: "Generating", variant: "blue" };
    case "likely_waiting":
      return { label: "Waiting", variant: "accent" };
    case "completed":
      return { label: "Completed", variant: "success" };
    case "failed":
    case "cancelled":
      return { label: "Failed", variant: "error" };
    case "idle":
      return { label: "Idle", variant: "muted" };
    default:
      return { label: status ?? "Unknown", variant: "muted" };
  }
}

function getRecord(value: unknown): Record<string, unknown> | null {
  return value != null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function extractRawText(result: unknown): string | null {
  if (typeof result === "string") {
    return result;
  }
  if (Array.isArray(result)) {
    const first = result[0];
    if (first != null && typeof first === "object" && typeof (first as { text?: unknown }).text === "string") {
      return (first as { text: string }).text;
    }
  }
  return null;
}

function renderSeverityBadges(parsed: Record<string, unknown>) {
  const gapCounts = getRecord(parsed["gap_counts"]);
  if (!gapCounts) return null;

  const entries: Array<{ label: string; key: string; variant: BadgeVariant }> = [
    { label: "C", key: "critical", variant: "error" },
    { label: "H", key: "high", variant: "accent" },
    { label: "M", key: "medium", variant: "warning" },
    { label: "L", key: "low", variant: "muted" },
  ];

  return (
    <WidgetRow compact>
      {entries.map(({ label, key, variant }) => {
        const count = getNumber(gapCounts, key) ?? 0;
        return (
          <Badge key={key} variant={variant} compact>
            {label} {count}
          </Badge>
        );
      })}
    </WidgetRow>
  );
}

function renderDelegateBadges(
  delegates: unknown[] | undefined,
  snapshots: unknown[] | undefined
) {
  if (!delegates || delegates.length === 0) return null;

  const snapshotByLabel = new Map<string, Record<string, unknown>>();
  for (const snapshot of snapshots ?? []) {
    const record = getRecord(snapshot);
    const label = getString(record, "label");
    if (label && record) {
      snapshotByLabel.set(label, record);
    }
  }

  return (
    <WidgetRow compact>
      {delegates.map((delegate, index) => {
        const record = getRecord(delegate);
        const label = getString(record, "label") ?? getString(record, "critic") ?? `delegate-${index + 1}`;
        const snapshot = snapshotByLabel.get(label);
        const status = agentStatusLabel(getString(snapshot, "status"));
        return (
          <Badge key={`${label}-${index}`} variant={status.variant} compact>
            {label}
          </Badge>
        );
      })}
    </WidgetRow>
  );
}

function VerificationCard(props: {
  compact?: boolean;
  icon: React.ReactNode;
  title: string;
  badge?: React.ReactNode;
  children: React.ReactNode;
}) {
  const compactProps = props.compact === undefined ? {} : { compact: props.compact };
  return (
    <WidgetCard
      {...compactProps}
      alwaysExpanded
      header={<WidgetHeader {...compactProps} icon={props.icon} title={props.title} badge={props.badge} />}
    >
      {props.children}
    </WidgetCard>
  );
}

function RunVerificationEnrichment({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const args = getRecord(toolCall.arguments) ?? {};
  const selectedSpecialists = getArray(parsed, "selected_specialists");
  const snapshots = getArray(parsed, "delegate_snapshots");
  const findings = getArray(parsed, "findings_by_critic");
  const requested = getArray(args, "selected_specialists");
  const timedOut = getBool(parsed, "timed_out") === true;

  if (selectedSpecialists == null && toolCall.result == null) {
    return (
      <InlineIndicator
        icon={<Sparkles size={11} style={{ color: colors.accent }} />}
        text="Starting verification enrichment..."
      />
    );
  }

  const specialistCount = selectedSpecialists?.length ?? 0;
  const foundCount = (findings ?? []).filter((entry) => getBool(entry, "found") === true).length;
  const compactProps = compact === undefined ? {} : { compact };

  return (
    <VerificationCard
      {...compactProps}
      icon={<Sparkles size={12} style={{ color: colors.accent }} />}
      title="Verification enrichment"
      badge={<Badge variant={timedOut ? "warning" : "blue"} compact>{timedOut ? "Timed out" : "Running"}</Badge>}
    >
      <WidgetRow compact={compact}>
        <Badge variant="blue" compact>{specialistCount} specialists</Badge>
        {foundCount > 0 && <Badge variant="success" compact>{foundCount} findings</Badge>}
        {Array.isArray(requested) && requested.length > 0 && (
          <Badge variant="muted" compact>{requested.length} requested</Badge>
        )}
      </WidgetRow>
      <WidgetRow compact={compact}>
        {(selectedSpecialists ?? []).map((specialist, index) => {
          const record = getRecord(specialist);
          const label = getString(record, "label") ?? getString(record, "name") ?? `specialist-${index + 1}`;
          const snapshot = (snapshots ?? [])
            .map((entry) => getRecord(entry))
            .find((entry) => getString(entry, "label") === label);
          const status = agentStatusLabel(getString(snapshot, "status"));
          return (
            <Badge key={`${label}-${index}`} variant={status.variant} compact>
              {label}
            </Badge>
          );
        })}
      </WidgetRow>
    </VerificationCard>
  );
}

function RunVerificationRound({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const args = getRecord(toolCall.arguments) ?? {};
  const round = getNumber(parsed, "round") ?? getNumber(args, "round");
  const classification = getString(parsed, "classification");
  const delegates = getArray(parsed, "required_delegates");
  const summary = getString(getRecord(parsed["required_critic_settlement"]), "summary");

  if (!classification && toolCall.result == null) {
    return (
      <InlineIndicator
        icon={<ListChecks size={11} style={{ color: colors.accent }} />}
        text={`Running verification round${round != null ? ` ${round}` : ""}...`}
      />
    );
  }

  const variant = statusBadgeVariant(classification);
  const compactProps = compact === undefined ? {} : { compact };

  return (
    <VerificationCard
      {...compactProps}
      icon={<ListChecks size={12} style={{ color: iconColorForVariant(variant) }} />}
      title="Verification round"
      badge={<Badge variant={variant} compact>{classification ?? "Running"}</Badge>}
    >
      <WidgetRow compact={compact}>
        {round != null && <Badge variant="blue" compact>{`Round ${round}`}</Badge>}
      </WidgetRow>
      {renderSeverityBadges(parsed)}
      {renderDelegateBadges(delegates, getArray(parsed, "delegate_snapshots"))}
      {summary && (
        <div style={{ fontSize: compact ? 10 : 10.5, color: colors.textMuted }}>
          {summary}
        </div>
      )}
    </VerificationCard>
  );
}

function RoundReport({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const status = getString(parsed, "status");
  const currentRound = getNumber(parsed, "current_round");
  const gaps = getArray(parsed, "current_gaps");
  const gapLabel = gaps?.length === 1 ? "1 gap" : `${gaps?.length ?? 0} gaps`;

  if (!status && toolCall.result == null) {
    return (
      <InlineIndicator
        icon={<ShieldCheck size={11} style={{ color: colors.blue }} />}
        text="Reporting verification round..."
      />
    );
  }

  const compactProps = compact === undefined ? {} : { compact };

  return (
    <VerificationCard
      {...compactProps}
      icon={<ShieldCheck size={12} style={{ color: iconColorForVariant(statusBadgeVariant(status)) }} />}
      title="Round report"
      badge={<Badge variant={statusBadgeVariant(status)} compact>{status ?? "Unknown"}</Badge>}
    >
      <WidgetRow compact={compact}>
        {currentRound != null && <Badge variant="blue" compact>{`Round ${currentRound}`}</Badge>}
        {gaps != null && <Badge variant={gaps.length > 0 ? "accent" : "success"} compact>{gapLabel}</Badge>}
      </WidgetRow>
    </VerificationCard>
  );
}

function CompleteVerification({ toolCall, compact }: ToolCallWidgetProps) {
  const parsed = parseMcpToolResult(toolCall.result);
  const raw = extractRawText(toolCall.result);
  const status = getString(parsed, "status");
  const settlement = getRecord(parsed["settlement"]);
  const settlementClassification = getString(settlement, "classification");
  const settlementSummary = getString(settlement, "summary");
  const convergence = convergenceLabel(getString(parsed, "convergence_reason"));

  if (!status && raw == null) {
    return (
      <InlineIndicator
        icon={<ShieldCheck size={11} style={{ color: colors.accent }} />}
        text="Finalizing verification..."
      />
    );
  }

  const variant =
    raw === "aborted"
      ? "warning"
      : statusBadgeVariant(settlementClassification ?? status);
  const compactProps = compact === undefined ? {} : { compact };

  return (
    <VerificationCard
      {...compactProps}
      icon={
        raw === "aborted"
          ? <ShieldAlert size={12} style={{ color: badgeStyles.warning.color }} />
          : <ShieldCheck size={12} style={{ color: iconColorForVariant(variant) }} />
      }
      title="Final cleanup"
      badge={<Badge variant={variant} compact>{raw === "aborted" ? "Aborted" : (settlementClassification ?? status ?? "Unknown")}</Badge>}
    >
      <WidgetRow compact={compact}>
        {status && <Badge variant={statusBadgeVariant(status)} compact>{status}</Badge>}
        {convergence && <Badge variant="muted" compact>{convergence}</Badge>}
      </WidgetRow>
      {settlementSummary && (
        <div style={{ fontSize: compact ? 10 : 10.5, color: colors.textMuted }}>
          {settlementSummary}
        </div>
      )}
      {raw === "aborted" && !settlementSummary && (
        <div style={{ fontSize: compact ? 10 : 10.5, color: colors.textMuted }}>
          Cleanup aborted before a canonical terminal result was returned.
        </div>
      )}
    </VerificationCard>
  );
}

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
  const session = getRecord(parsed.session);
  const agentState = getRecord(parsed.agent_state);
  const verification = getRecord(parsed.verification);

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
      {title && <span style={truncatedTitleStyle(compact)}>{title}</span>}
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
  const raw = parseMcpToolResultRaw(toolCall.result);
  const sessions = Array.isArray(raw) ? raw : getArray(parseMcpToolResult(toolCall.result), "sessions");

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

export const VerificationWidget = React.memo(function VerificationWidget(props: ToolCallWidgetProps) {
  const toolType = getToolType(props.toolCall.name);

  switch (toolType) {
    case "run_verification_enrichment":
      return <div data-testid="verification-widget-enrichment"><RunVerificationEnrichment {...props} /></div>;
    case "run_verification_round":
      return <div data-testid="verification-widget-round"><RunVerificationRound {...props} /></div>;
    case "report_verification_round":
      return <div data-testid="verification-widget-round-report"><RoundReport {...props} /></div>;
    case "complete_plan_verification":
      return <div data-testid="verification-widget-complete"><CompleteVerification {...props} /></div>;
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
