/**
 * McpContextWidgets — Widgets for MCP context/session/memory/team-plan tools.
 *
 * Handles:
 * - mcp__ralphx__get_parent_session_context — "Session Context" + success badge
 * - mcp__ralphx__get_team_session_state — "Team State" + badge
 * - mcp__ralphx__search_memories — query text + result count badge
 * - mcp__ralphx__request_team_plan — team plan card with teammate list
 *
 * Context/session/memory tools are plumbing — kept as compact as possible (single line).
 * request_team_plan is a WidgetCard with teammate details.
 */

import React, { useMemo } from "react";
import { Database, Users, Search } from "lucide-react";
import { InlineIndicator, Badge, WidgetCard, WidgetHeader } from "./shared";
import { colors, getString, getArray, parseToolResultAsLines } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";

// ============================================================================
// SessionContextWidget — mcp__ralphx__get_parent_session_context
// ============================================================================

export const SessionContextWidget = React.memo(function SessionContextWidget({
  toolCall,
  className,
}: ToolCallWidgetProps) {
  const hasResult = toolCall.result != null;
  const hasError = Boolean(toolCall.error);

  return (
    <div
      className={className}
      style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 0", margin: "2px 0" }}
    >
      <Database size={12} style={{ color: colors.textMuted, flexShrink: 0 }} />
      <span style={{ fontSize: 10.5, color: colors.textSecondary }}>Session Context</span>
      {hasError ? (
        <Badge variant="error" compact>error</Badge>
      ) : hasResult ? (
        <Badge variant="success" compact>loaded</Badge>
      ) : (
        <Badge variant="muted" compact>loading</Badge>
      )}
    </div>
  );
});

// ============================================================================
// TeamSessionStateWidget — mcp__ralphx__get_team_session_state
// ============================================================================

export const TeamSessionStateWidget = React.memo(function TeamSessionStateWidget({
  toolCall,
  className,
}: ToolCallWidgetProps) {
  const hasResult = toolCall.result != null;
  const hasError = Boolean(toolCall.error);

  return (
    <div
      className={className}
      style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 0", margin: "2px 0" }}
    >
      <Users size={12} style={{ color: colors.textMuted, flexShrink: 0 }} />
      <span style={{ fontSize: 10.5, color: colors.textSecondary }}>Team State</span>
      {hasError ? (
        <Badge variant="error" compact>error</Badge>
      ) : hasResult ? (
        <Badge variant="success" compact>loaded</Badge>
      ) : (
        <Badge variant="muted" compact>loading</Badge>
      )}
    </div>
  );
});

// ============================================================================
// SearchMemoriesWidget — mcp__ralphx__search_memories
// ============================================================================

/** Count results from search_memories response */
function countResults(result: unknown): number | null {
  if (result == null) return null;
  if (Array.isArray(result)) {
    // MCP wrapper: [{type: "text", text: "..."}]
    const first = result[0];
    if (first && typeof first === "object" && "text" in first) {
      const text = String((first as { text: string }).text);
      const lines = text.split("\n").filter(Boolean);
      return lines.length;
    }
    return result.length;
  }
  if (typeof result === "string") {
    return result.split("\n").filter(Boolean).length;
  }
  return null;
}

export const SearchMemoriesWidget = React.memo(function SearchMemoriesWidget({
  toolCall,
  className,
}: ToolCallWidgetProps) {
  const query = getString(toolCall.arguments, "query");
  const hasResult = toolCall.result != null;
  const hasError = Boolean(toolCall.error);
  const resultCount = hasResult ? countResults(toolCall.result) : null;

  return (
    <div
      className={className}
      style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 0", margin: "2px 0" }}
    >
      <Search size={12} style={{ color: colors.textMuted, flexShrink: 0 }} />
      <span style={{ fontSize: 10.5, color: colors.textSecondary }}>Search Memories</span>
      {query && (
        <span
          style={{
            fontSize: 10,
            color: colors.textMuted,
            overflow: "hidden",
            textOverflow: "ellipsis",
            whiteSpace: "nowrap",
            maxWidth: 180,
            fontFamily: "var(--font-mono)",
          }}
          title={query}
        >
          {query}
        </span>
      )}
      {hasError ? (
        <Badge variant="error" compact>error</Badge>
      ) : resultCount != null ? (
        <Badge variant="muted" compact>{resultCount} results</Badge>
      ) : hasResult ? (
        <Badge variant="success" compact>done</Badge>
      ) : (
        <InlineIndicator text="" />
      )}
    </div>
  );
});

// ============================================================================
// TeamPlanWidget — mcp__ralphx__request_team_plan
// ============================================================================

interface TeammateEntry {
  role: string;
  model: string;
  promptSummary: string;
}

/** Extract teammate entries from tool call arguments */
function parseTeammates(args: unknown): TeammateEntry[] {
  const raw = getArray(args, "teammates");
  if (!raw) return [];
  const entries: TeammateEntry[] = [];
  for (const item of raw) {
    if (item && typeof item === "object") {
      const role = getString(item, "role") ?? "teammate";
      const model = getString(item, "model") ?? "sonnet";
      const promptSummary = getString(item, "prompt_summary") ?? "";
      entries.push({ role, model, promptSummary });
    }
  }
  return entries;
}

/** Extract spawned count from result (MCP text wrapper or direct object) */
function parseSpawnResult(result: unknown): { planId: string | null; spawnedCount: number | null } {
  if (result == null) return { planId: null, spawnedCount: null };

  // Try direct object fields first
  const planId = getString(result, "plan_id");
  const teammates = getArray(result, "teammates_spawned");
  if (planId) {
    return { planId, spawnedCount: teammates?.length ?? null };
  }

  // Try MCP text wrapper — parse "X teammates spawned" or similar
  const lines = parseToolResultAsLines(result);
  for (const line of lines) {
    const planMatch = line.match(/plan_id[:\s]+(\S+)/i);
    if (planMatch?.[1]) {
      const countMatch = lines.join(" ").match(/(\d+)\s*(?:\/\s*\d+\s*)?teammates?\s+(?:spawned|registered)/i);
      return { planId: planMatch[1], spawnedCount: countMatch?.[1] ? parseInt(countMatch[1], 10) : null };
    }
  }

  return { planId: null, spawnedCount: null };
}

const MODEL_VARIANT: Record<string, "muted" | "accent" | "blue"> = {
  haiku: "muted",
  sonnet: "blue",
  opus: "accent",
};

export const TeamPlanWidget = React.memo(function TeamPlanWidget({
  toolCall,
  compact = false,
  className,
}: ToolCallWidgetProps) {
  const args = toolCall.arguments;
  const process = getString(args, "process") ?? "team";
  const teamName = getString(args, "team_name");
  const teammates = useMemo(() => parseTeammates(args), [args]);
  const { planId, spawnedCount } = useMemo(() => parseSpawnResult(toolCall.result), [toolCall.result]);
  const hasError = Boolean(toolCall.error);

  const headerTitle = teamName ? `Team Plan — ${process}` : `Team Plan — ${process}`;

  const resultBadge = hasError ? (
    <Badge variant="error" compact>error</Badge>
  ) : spawnedCount != null ? (
    <Badge variant="success" compact>{spawnedCount} spawned</Badge>
  ) : planId ? (
    <Badge variant="blue" compact>pending</Badge>
  ) : teammates.length > 0 ? (
    <Badge variant="muted" compact>{teammates.length} teammates</Badge>
  ) : undefined;

  // No teammates to show — inline fallback
  if (teammates.length === 0) {
    return (
      <div
        className={className}
        style={{ display: "flex", alignItems: "center", gap: 6, padding: "2px 0", margin: "2px 0" }}
      >
        <Users size={12} style={{ color: colors.accent, flexShrink: 0 }} />
        <span style={{ fontSize: 10.5, color: colors.textSecondary }}>{headerTitle}</span>
        {resultBadge}
      </div>
    );
  }

  return (
    <WidgetCard
      className={className ?? ""}
      compact={compact}
      defaultExpanded={false}
      header={
        <WidgetHeader
          icon={<Users size={14} style={{ color: colors.accent }} />}
          title={headerTitle}
          badge={resultBadge}
          compact={compact}
        />
      }
    >
      {teammates.map((mate, i) => (
        <div
          key={i}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "2px 0",
            fontSize: compact ? 10.5 : 11,
          }}
        >
          <span style={{ color: colors.textSecondary, fontWeight: 500, minWidth: 80 }}>
            {mate.role}
          </span>
          <Badge variant={MODEL_VARIANT[mate.model] ?? "muted"} compact>
            {mate.model}
          </Badge>
          {mate.promptSummary && (
            <span
              style={{
                color: colors.textMuted,
                fontSize: 10,
                flex: 1,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
              title={mate.promptSummary}
            >
              {mate.promptSummary}
            </span>
          )}
        </div>
      ))}
    </WidgetCard>
  );
});
