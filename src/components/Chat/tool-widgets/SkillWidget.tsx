/**
 * SkillWidget — Skill Tool Call Card
 *
 * Renders specialized card for Skill tool calls (e.g., /commit, /review-pr).
 *
 * - Header: skill icon + skill argument (e.g., ralphx:rule-manager) + status badge
 * - Body: result text preview (collapsible)
 * - Error state: error badge + error content
 */

import React, { useMemo } from "react";
import { Zap } from "lucide-react";

import { WidgetCard, WidgetHeader, Badge } from "./shared";
import { colors, parseToolResultAsLines } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";

// ============================================================================
// Helpers
// ============================================================================

interface SkillArgs {
  skill?: string;
  args?: string;
}

interface ParsedSkill {
  skillName: string;
  skillArgs: string;
  resultText: string;
  hasError: boolean;
  errorText: string;
}

/** Extract skill name, args, result from tool call data */
function parseSkillToolCall(toolCall: ToolCallWidgetProps["toolCall"]): ParsedSkill {
  const args = (toolCall.arguments ?? {}) as SkillArgs;
  const skillName = args.skill ?? "";
  const skillArgs = args.args ?? "";

  // Error handling
  const hasError = Boolean(toolCall.error);
  const errorText = toolCall.error ?? "";

  // Result parsing
  let resultText = "";
  if (hasError) {
    resultText = errorText;
  } else if (toolCall.result) {
    const lines = parseToolResultAsLines(toolCall.result);
    resultText = lines.join("\n");
  }

  return { skillName, skillArgs, resultText, hasError, errorText };
}

// ============================================================================
// Component
// ============================================================================

export const SkillWidget = React.memo(function SkillWidget({
  toolCall,
  compact = false,
  className = "",
}: ToolCallWidgetProps) {
  const parsed = useMemo(() => parseSkillToolCall(toolCall), [toolCall]);

  // Header title: skill name with args if present
  const headerTitle = parsed.skillArgs
    ? `${parsed.skillName} ${parsed.skillArgs}`
    : parsed.skillName || "Skill";

  // Status badge
  const statusBadge = parsed.hasError ? (
    <Badge variant="error" compact>
      error
    </Badge>
  ) : toolCall.result != null ? (
    <Badge variant="success" compact>
      ok
    </Badge>
  ) : null;

  return (
    <WidgetCard
      className={className}
      compact={compact}
      defaultExpanded={parsed.hasError}
      header={
        <WidgetHeader
          icon={<Zap size={14} />}
          title={headerTitle}
          badge={statusBadge}
          compact={compact}
          mono
        />
      }
    >
      {/* Result or error text */}
      {parsed.resultText && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: compact ? 10 : 11,
            lineHeight: 1.55,
            color: parsed.hasError ? colors.error : colors.textSecondary,
            background: parsed.hasError ? colors.errorDim : colors.bgTerminal,
            borderRadius: 4,
            padding: "6px 8px",
            marginTop: 4,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {parsed.resultText}
        </div>
      )}
    </WidgetCard>
  );
});
