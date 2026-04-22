/**
 * ChildSessionWidget — Live chat card for mcp__ralphx__create_child_session tool calls.
 *
 * Header (always visible): GitBranch icon + title + purpose badge + agent status badge.
 * Collapsed body: latest message snippet (80 chars plain text).
 * Expanded body: LoadingSkeleton → MessagePreviewList (5 messages) + "Open Session" button.
 *
 * Polling: 5s when agent is active, disabled when idle.
 * History guard: if first fetch returns idle + no messages, polling is permanently disabled.
 */

import React, { useContext } from "react";
import { GitBranch, User, Bot, AlertCircle } from "lucide-react";
import { WidgetCard, WidgetHeader, InlineIndicator, Badge, WidgetRow } from "./shared";
import {
  colors,
  getString,
  getBool,
  parseMcpToolResult,
  truncate,
  truncatedTitleStyle,
} from "./shared.constants";
import type { ToolCallWidgetProps, BadgeVariant } from "./shared.constants";
import { ChildSessionNavigationContext } from "./ChildSessionNavigationContext";
import { useChildSessionStatus } from "@/hooks/useChildSessionStatus";
import { formatRelativeTime } from "@/lib/formatters";
import { withAlpha } from "@/lib/theme-colors";

// ============================================================================
// Helpers
// ============================================================================

function stripMarkdown(text: string): string {
  return text
    .replace(/```[\s\S]*?```/g, "[code]")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    .replace(/^#+\s+/gm, "")
    .replace(/\n+/g, " ")
    .trim();
}

// ============================================================================
// Sub-components
// ============================================================================

function AgentStatusBadge({
  status,
}: {
  status: "idle" | "likely_generating" | "likely_waiting";
}) {
  if (status === "likely_generating") {
    return <Badge variant="success" compact>Generating</Badge>;
  }
  if (status === "likely_waiting") {
    return <Badge variant="blue" compact>Waiting</Badge>;
  }
  return null;
}

function LoadingSkeleton() {
  const lineStyle: React.CSSProperties = {
    height: 12,
    borderRadius: 4,
    backgroundColor: withAlpha("var(--bg-hover)", 60),
    marginBottom: 8,
  };
  return (
    <div style={{ padding: "4px 0 0" }} aria-label="Loading messages">
      <div style={{ ...lineStyle, width: "85%" }} />
      <div style={{ ...lineStyle, width: "70%" }} />
      <div style={{ ...lineStyle, width: "55%", marginBottom: 0 }} />
    </div>
  );
}

function MessagePreviewItem({
  role,
  content,
  createdAt,
}: {
  role: string;
  content: string;
  createdAt: string | null;
}) {
  const isUser = role === "user";
  const Icon = isUser ? User : Bot;
  const iconColor = isUser ? colors.textMuted : colors.blue;
  const preview = truncate(stripMarkdown(content), 120);
  const timestamp = createdAt ? formatRelativeTime(createdAt) : null;

  return (
    <div
      style={{
        display: "flex",
        gap: 6,
        alignItems: "flex-start",
        padding: "4px 0",
        borderBottom: `1px solid ${withAlpha("var(--bg-hover)", 40)}`,
      }}
    >
      <Icon size={11} style={{ color: iconColor, flexShrink: 0, marginTop: 2 }} />
      <div style={{ flex: 1, minWidth: 0 }}>
        <span
          style={{
            fontSize: 12,
            color: "var(--text-secondary)",
            lineHeight: 1.4,
            display: "block",
            wordBreak: "break-word",
          }}
        >
          {preview}
        </span>
        {timestamp && (
          <span
            style={{ fontSize: 10, color: colors.textMuted, display: "block", marginTop: 2 }}
          >
            {timestamp}
          </span>
        )}
      </div>
    </div>
  );
}

function ErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 6,
        padding: "6px 0",
        color: colors.textMuted,
        fontSize: 12,
      }}
      aria-label="Unable to load session"
    >
      <AlertCircle size={11} style={{ flexShrink: 0 }} />
      <span>Unable to load session.</span>
      <button
        onClick={onRetry}
        style={{
          background: "none",
          border: "none",
          padding: 0,
          cursor: "pointer",
          color: colors.blue,
          fontSize: 12,
          textDecoration: "underline",
        }}
      >
        Retry
      </button>
    </div>
  );
}

// ============================================================================
// ChildSessionWidget
// ============================================================================

export const ChildSessionWidget = React.memo(function ChildSessionWidget({
  toolCall,
  compact,
}: ToolCallWidgetProps) {
  const isProjectIdeationRun = toolCall.name.toLowerCase().includes("start_ideation_session");
  const parsed = parseMcpToolResult(toolCall.result);
  const title =
    getString(toolCall.arguments, "title") ??
    getString(parsed, "title") ??
    (isProjectIdeationRun ? "Ideation run" : undefined);
  const purpose =
    getString(toolCall.arguments, "purpose") ??
    getString(parsed, "purpose") ??
    (isProjectIdeationRun ? "ideation" : undefined);
  const orchestrationTriggered =
    getBool(parsed, "orchestration_triggered") ?? getBool(parsed, "agent_spawned");
  const sessionId =
    getString(parsed, "session_id") ??
    getString(parsed, "sessionId") ??
    getString(parsed, "child_session_id") ??
    getString(parsed, "childSessionId");

  const onNavigate = useContext(ChildSessionNavigationContext);

  const { data, isLoading, isError, refetch } = useChildSessionStatus(sessionId);

  if (!title) {
    return (
      <InlineIndicator
        icon={<GitBranch size={11} style={{ color: colors.blue }} />}
        text="Creating session..."
      />
    );
  }

  const purposeVariant: BadgeVariant = purpose === "verification" ? "blue" : "muted";
  const agentStatus = data?.agent_state.estimated_status ?? "idle";
  const isPendingCapacity = !!(data?.pending_initial_prompt);
  const latestMessage = data?.recent_messages[data.recent_messages.length - 1];
  const snippet = latestMessage ? truncate(stripMarkdown(latestMessage.content), 80) : null;
  const visualState =
    isLoading
      ? "loading"
      : isError
        ? "error"
        : isPendingCapacity
          ? "pending"
          : agentStatus === "idle"
            ? "idle"
            : "active";

  return (
    <div data-testid={`child-session-widget-${visualState}`}>
      <WidgetCard
        {...(compact !== undefined && { compact })}
        defaultExpanded={false}
        header={
          <WidgetHeader
            icon={<GitBranch size={12} />}
            title={
              purpose === "verification"
                ? "Verification Session"
                : isProjectIdeationRun
                  ? "Ideation Session"
                  : "Follow-up Session"
            }
            {...(compact !== undefined && { compact })}
            badge={
              <>
                {purpose && <Badge variant={purposeVariant} compact>{purpose}</Badge>}
                {orchestrationTriggered === true && (
                  <Badge variant="success" compact>Agent spawned</Badge>
                )}
                {isPendingCapacity && agentStatus === "idle" && (
                  <Badge variant="warning" compact>Waiting for capacity</Badge>
                )}
                {!isPendingCapacity && agentStatus !== "idle" && <AgentStatusBadge status={agentStatus} />}
                {sessionId && (
                  <button
                    onClick={(e) => { e.stopPropagation(); onNavigate(sessionId); }}
                    onKeyDown={(e) => { e.stopPropagation(); }}
                    style={{
                      padding: "2px 8px",
                      fontSize: 11,
                      cursor: "pointer",
                      border: `1px solid ${colors.accentBorder}`,
                      borderRadius: 4,
                      backgroundColor: colors.accentDim,
                      color: colors.accent,
                      lineHeight: 1.4,
                    }}
                    aria-label="Open Session"
                  >
                    Open Run
                  </button>
                )}
              </>
            }
          />
        }
      >
        {/* Full session title — always visible in body */}
        <span
          style={{
            display: "block",
            fontSize: 12,
            color: colors.textPrimary,
            wordBreak: "break-word",
            marginBottom: 4,
          }}
        >
          {title}
        </span>

        {/* Collapsed body: snippet (single line — stable height) */}
        <WidgetRow compact={compact}>
          <span
            style={{
              ...truncatedTitleStyle(compact),
              fontSize: 11,
              color: (snippet || isPendingCapacity) ? colors.textMuted : "transparent",
            }}
          >
            {snippet ?? (isPendingCapacity ? "Waiting for capacity..." : "No messages yet")}
          </span>
        </WidgetRow>

        {/* Expanded body (visible when card is open) */}
        {isLoading && <LoadingSkeleton />}
        {isError && <ErrorState onRetry={() => void refetch()} />}
        {!isLoading && !isError && data && data.recent_messages.length > 0 && (
          <div style={{ marginTop: 4 }}>
            {data.recent_messages.map((msg, idx) => (
              <MessagePreviewItem
                key={idx}
                role={msg.role}
                content={msg.content}
                createdAt={msg.created_at}
              />
            ))}
          </div>
        )}
      </WidgetCard>
    </div>
  );
});
