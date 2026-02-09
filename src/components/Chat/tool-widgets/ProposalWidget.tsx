/**
 * ProposalWidget — Compact card for proposal CRUD tool calls
 *
 * Handles: create_task_proposal, update_task_proposal, delete_task_proposal
 *
 * Design:
 * - create: title + category badge + "Created" indicator
 * - update: title + "Updated" badge with changed fields summary
 * - delete: "Deleted" badge with proposal title
 */

import React from "react";
import { Plus, Pencil, Trash2 } from "lucide-react";
import { InlineIndicator, Badge } from "./shared";
import { colors } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";

// ============================================================================
// Helpers
// ============================================================================

type ProposalAction = "created" | "updated" | "deleted";

function getAction(toolName: string): ProposalAction {
  const name = toolName.toLowerCase();
  if (name.includes("delete")) return "deleted";
  if (name.includes("update")) return "updated";
  return "created";
}

function getString(obj: unknown, key: string): string | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return typeof val === "string" ? val : undefined;
  }
  return undefined;
}

/** Extract proposal title from args or result */
function extractTitle(toolCall: ToolCallWidgetProps["toolCall"]): string {
  // Try result first (has the canonical title after create/update)
  const fromResult = getString(toolCall.result, "title");
  if (fromResult) return fromResult;

  // Args for create/update
  const fromArgs = getString(toolCall.arguments, "title");
  if (fromArgs) return fromArgs;

  return "Proposal";
}

/** Extract category from args or result */
function extractCategory(toolCall: ToolCallWidgetProps["toolCall"]): string | undefined {
  return getString(toolCall.result, "category") ?? getString(toolCall.arguments, "category");
}

/** For update: build summary of which fields changed */
function extractChangedFields(toolCall: ToolCallWidgetProps["toolCall"]): string[] {
  const args = toolCall.arguments;
  if (args == null || typeof args !== "object") return [];

  const fields: string[] = [];
  const a = args as Record<string, unknown>;
  if (a.title != null) fields.push("title");
  if (a.description != null) fields.push("description");
  if (a.category != null) fields.push("category");
  if (a.user_priority != null) fields.push("priority");
  if (a.steps != null) fields.push("steps");
  if (a.acceptance_criteria != null) fields.push("criteria");
  return fields;
}

// ============================================================================
// Action config
// ============================================================================

const actionConfig: Record<ProposalAction, {
  icon: React.ReactNode;
  label: string;
  badgeVariant: "success" | "blue" | "error";
  color: string;
}> = {
  created: {
    icon: <Plus size={12} />,
    label: "Created",
    badgeVariant: "success",
    color: colors.success,
  },
  updated: {
    icon: <Pencil size={11} />,
    label: "Updated",
    badgeVariant: "blue",
    color: colors.blue,
  },
  deleted: {
    icon: <Trash2 size={11} />,
    label: "Deleted",
    badgeVariant: "error",
    color: colors.error,
  },
};

// ============================================================================
// ProposalWidget
// ============================================================================

export const ProposalWidget = React.memo(function ProposalWidget({
  toolCall,
  compact = false,
}: ToolCallWidgetProps) {
  const action = getAction(toolCall.name);
  const config = actionConfig[action];
  const title = extractTitle(toolCall);
  const category = extractCategory(toolCall);
  const changedFields = action === "updated" ? extractChangedFields(toolCall) : [];

  // For delete with no title found, show a minimal indicator
  if (action === "deleted" && title === "Proposal") {
    return (
      <InlineIndicator
        icon={<Trash2 size={11} style={{ color: colors.error }} />}
        text="Proposal deleted"
      />
    );
  }

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 7,
        padding: compact ? "3px 10px" : "5px 10px",
        margin: "2px 0",
      }}
    >
      {/* Action icon */}
      <span
        style={{
          width: 14,
          height: 14,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          flexShrink: 0,
          color: config.color,
        }}
      >
        {config.icon}
      </span>

      {/* Title (truncated) */}
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: action === "deleted" ? colors.textMuted : colors.textSecondary,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          textDecoration: action === "deleted" ? "line-through" : undefined,
        }}
      >
        {title}
      </span>

      {/* Category chip (only for create/update) */}
      {category && action !== "deleted" && (
        <Badge variant="accent" compact>{category}</Badge>
      )}

      {/* Changed fields summary (only for update) */}
      {changedFields.length > 0 && (
        <span
          style={{
            fontSize: 10,
            color: colors.textMuted,
            flexShrink: 0,
            whiteSpace: "nowrap",
          }}
        >
          {changedFields.join(", ")}
        </span>
      )}

      {/* Action badge */}
      <Badge variant={config.badgeVariant} compact>{config.label}</Badge>
    </div>
  );
});
