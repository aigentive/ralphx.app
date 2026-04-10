/**
 * SendMessageWidget — Team Message Card
 *
 * Renders SendMessage tool calls with type badge, recipient pill, and content preview.
 * Types: message, broadcast, shutdown_request, shutdown_response, plan_approval_response
 */

import React, { useMemo } from "react";
import { MessageSquare } from "lucide-react";

import { WidgetCard, WidgetHeader, Badge } from "./shared";
import { colors, getString, getBool } from "./shared.constants";
import type { ToolCallWidgetProps, BadgeVariant } from "./shared.constants";

// ============================================================================
// Types & Constants
// ============================================================================

type MessageType =
  | "message"
  | "broadcast"
  | "shutdown_request"
  | "shutdown_response"
  | "plan_approval_response";

interface TypeConfig {
  label: string;
  variant: BadgeVariant;
}

const TYPE_CONFIG: Record<MessageType, TypeConfig> = {
  message: { label: "message", variant: "blue" },
  broadcast: { label: "broadcast", variant: "accent" },
  shutdown_request: { label: "shutdown", variant: "muted" },
  shutdown_response: { label: "shutdown", variant: "muted" },
  plan_approval_response: { label: "plan approval", variant: "success" },
};

const FALLBACK_CONFIG: TypeConfig = { label: "message", variant: "muted" };

// ============================================================================
// Helpers
// ============================================================================

interface ParsedSendMessage {
  type: string;
  typeConfig: TypeConfig;
  recipient: string | undefined;
  content: string;
  summary: string | undefined;
  approve: boolean | undefined;
}

function parseSendMessage(
  toolCall: ToolCallWidgetProps["toolCall"],
): ParsedSendMessage {
  const args = toolCall.arguments;
  const type = getString(args, "type") ?? "message";
  const typeConfig =
    TYPE_CONFIG[type as MessageType] ?? FALLBACK_CONFIG;
  const recipient = getString(args, "recipient");
  const content = getString(args, "content") ?? "";
  const summary = getString(args, "summary");
  const approve = getBool(args, "approve");

  return { type, typeConfig, recipient, content, summary, approve };
}

/** Truncate text to a max number of lines */
function truncateLines(text: string, maxLines: number): string {
  const lines = text.split("\n");
  if (lines.length <= maxLines) return text;
  return lines.slice(0, maxLines).join("\n") + "\u2026";
}

// ============================================================================
// Component
// ============================================================================

export const SendMessageWidget = React.memo(function SendMessageWidget({
  toolCall,
  compact = false,
  className = "",
}: ToolCallWidgetProps) {
  const parsed = useMemo(() => parseSendMessage(toolCall), [toolCall]);

  const headerTitle = parsed.recipient
    ? `to ${parsed.recipient}`
    : parsed.type === "broadcast"
      ? "to all"
      : "Send message";

  const typeBadge = (
    <Badge variant={parsed.typeConfig.variant} compact>
      {parsed.typeConfig.label}
    </Badge>
  );

  const previewText = parsed.summary ?? truncateLines(parsed.content, 3);

  return (
    <div data-testid={`send-message-widget-${parsed.type}`}>
      <WidgetCard
        className={className}
        compact={compact}
        defaultExpanded={false}
        header={
          <WidgetHeader
            icon={<MessageSquare size={14} />}
            title={headerTitle}
            badge={
              <span style={{ display: "flex", alignItems: "center", gap: 4 }}>
                {parsed.recipient && parsed.type === "broadcast" && (
                  <RecipientPill name="all" compact={compact} />
                )}
                {typeBadge}
              </span>
            }
            compact={compact}
          />
        }
      >
        {/* Preview text */}
        {previewText && (
          <div
            style={{
              fontSize: compact ? 10.5 : 11,
              lineHeight: 1.5,
              color: colors.textSecondary,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              padding: "2px 0",
            }}
          >
            {previewText}
          </div>
        )}

        {/* Full content (shown when expanded, only if different from preview) */}
        {parsed.summary && parsed.content && parsed.content !== parsed.summary && (
          <div
            style={{
              fontSize: compact ? 10 : 10.5,
              lineHeight: 1.5,
              color: colors.textMuted,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              marginTop: 4,
              padding: "4px 6px",
              background: colors.bgTerminal,
              borderRadius: 4,
            }}
          >
            {parsed.content}
          </div>
        )}

        {/* Approve indicator for shutdown/plan responses */}
        {parsed.approve !== undefined && (
          <div
            style={{
              fontSize: compact ? 10 : 10.5,
              color: parsed.approve ? colors.success : colors.error,
              marginTop: 4,
            }}
          >
            {parsed.approve ? "Approved" : "Rejected"}
          </div>
        )}

        {/* Result display */}
        {toolCall.result != null && !toolCall.error && (
          <ResultDisplay result={toolCall.result} compact={compact} />
        )}

        {/* Error display */}
        {toolCall.error && (
          <div
            style={{
              fontSize: compact ? 10 : 10.5,
              color: colors.error,
              marginTop: 4,
            }}
          >
            {toolCall.error}
          </div>
        )}
      </WidgetCard>
    </div>
  );
});

// ============================================================================
// Sub-components
// ============================================================================

function RecipientPill({
  name,
  compact = false,
}: {
  name: string;
  compact?: boolean;
}) {
  return (
    <span
      style={{
        fontSize: compact ? 9.5 : 10,
        padding: "1px 5px",
        borderRadius: 8,
        background: colors.bgElevated,
        color: colors.textSecondary,
        border: `1px solid ${colors.borderSubtle}`,
        whiteSpace: "nowrap",
      }}
    >
      @{name}
    </span>
  );
}

function ResultDisplay({
  result,
  compact = false,
}: {
  result: unknown;
  compact?: boolean;
}) {
  const text = useMemo(() => {
    if (typeof result === "string") return result;
    if (Array.isArray(result)) {
      const first = result[0];
      if (
        first &&
        typeof first === "object" &&
        "text" in first &&
        typeof (first as { text: unknown }).text === "string"
      ) {
        return (first as { text: string }).text;
      }
    }
    if (result && typeof result === "object" && "text" in result) {
      return String((result as { text: string }).text);
    }
    return null;
  }, [result]);

  if (!text) return null;

  return (
    <div
      style={{
        fontSize: compact ? 10 : 10.5,
        color: colors.textMuted,
        marginTop: 4,
        fontStyle: "italic",
      }}
    >
      {text.length > 100 ? text.slice(0, 100) + "\u2026" : text}
    </div>
  );
}
