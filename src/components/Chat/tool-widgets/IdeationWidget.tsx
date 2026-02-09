/**
 * IdeationWidget — Compact indicators for ideation session tools
 *
 * Handles:
 * - create_plan_artifact: plan title + "Plan created"
 * - update_plan_artifact: "Plan updated"
 * - link_proposals_to_plan: count of linked proposals
 * - ask_user_question: question text preview
 * - list_session_proposals: proposal count
 * - get_proposal: summary card
 * - get_session_plan: summary card
 * - analyze_session_dependencies: summary card
 */

import React from "react";
import {
  FileText,
  Link,
  MessageCircleQuestion,
  List,
  Search,
  GitBranch,
} from "lucide-react";
import { InlineIndicator, Badge } from "./shared";
import { colors } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";

// ============================================================================
// Helpers
// ============================================================================

function getString(obj: unknown, key: string): string | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return typeof val === "string" ? val : undefined;
  }
  return undefined;
}

function getNumber(obj: unknown, key: string): number | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return typeof val === "number" ? val : undefined;
  }
  return undefined;
}

function getArray(obj: unknown, key: string): unknown[] | undefined {
  if (obj != null && typeof obj === "object" && key in (obj as Record<string, unknown>)) {
    const val = (obj as Record<string, unknown>)[key];
    return Array.isArray(val) ? val : undefined;
  }
  return undefined;
}

type IdeationTool =
  | "create_plan_artifact"
  | "update_plan_artifact"
  | "link_proposals_to_plan"
  | "ask_user_question"
  | "list_session_proposals"
  | "get_proposal"
  | "get_session_plan"
  | "analyze_session_dependencies";

function getToolType(toolName: string): IdeationTool | null {
  const name = toolName.toLowerCase();
  if (name.includes("create_plan_artifact")) return "create_plan_artifact";
  if (name.includes("update_plan_artifact")) return "update_plan_artifact";
  if (name.includes("link_proposals_to_plan")) return "link_proposals_to_plan";
  if (name.includes("ask_user_question")) return "ask_user_question";
  if (name.includes("list_session_proposals")) return "list_session_proposals";
  if (name.includes("get_proposal")) return "get_proposal";
  if (name.includes("get_session_plan")) return "get_session_plan";
  if (name.includes("analyze_session_dependencies")) return "analyze_session_dependencies";
  return null;
}

// ============================================================================
// Sub-renderers
// ============================================================================

function PlanCreated({ toolCall, compact }: ToolCallWidgetProps) {
  const title = getString(toolCall.result, "name")
    ?? getString(toolCall.arguments, "title")
    ?? "Plan";

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
      <FileText size={12} style={{ color: colors.accent, flexShrink: 0 }} />
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: colors.textSecondary,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {title}
      </span>
      <Badge variant="success" compact>Plan created</Badge>
    </div>
  );
}

function PlanUpdated({ compact }: ToolCallWidgetProps) {
  return (
    <InlineIndicator
      icon={<FileText size={11} style={{ color: colors.blue }} />}
      text={compact ? "Plan updated" : "Plan artifact updated"}
    />
  );
}

function LinkProposals({ toolCall, compact }: ToolCallWidgetProps) {
  // Count from args (proposal_ids array)
  const proposalIds = getArray(toolCall.arguments, "proposal_ids");
  const count = proposalIds?.length;

  const text = count != null
    ? `${count} proposal${count !== 1 ? "s" : ""} linked to plan`
    : "Proposals linked to plan";

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
      <Link size={11} style={{ color: colors.blue, flexShrink: 0 }} />
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: colors.textSecondary,
        }}
      >
        {text}
      </span>
      {count != null && <Badge variant="blue" compact>{count}</Badge>}
    </div>
  );
}

function AskUserQuestion({ toolCall, compact }: ToolCallWidgetProps) {
  const question = getString(toolCall.arguments, "question");
  const header = getString(toolCall.arguments, "header");
  const preview = header ?? question ?? "Asking user...";

  // Truncate long questions
  const maxLen = compact ? 60 : 80;
  const truncated = preview.length > maxLen
    ? preview.slice(0, maxLen) + "..."
    : preview;

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
      <MessageCircleQuestion size={12} style={{ color: colors.accent, flexShrink: 0 }} />
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: colors.textSecondary,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          fontStyle: "italic",
        }}
      >
        {truncated}
      </span>
      <Badge variant="accent" compact>Question</Badge>
    </div>
  );
}

function ListProposals({ toolCall, compact }: ToolCallWidgetProps) {
  const count = getNumber(toolCall.result, "count");
  const proposals = getArray(toolCall.result, "proposals");
  const n = count ?? proposals?.length;

  if (n == null) {
    return <InlineIndicator icon={<List size={11} style={{ color: colors.textMuted }} />} text="Loading proposals..." />;
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
      <List size={12} style={{ color: colors.textMuted, flexShrink: 0 }} />
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: colors.textSecondary,
        }}
      >
        {n} proposal{n !== 1 ? "s" : ""} in session
      </span>
      <Badge variant="muted" compact>{n}</Badge>
    </div>
  );
}

function GetProposal({ toolCall, compact }: ToolCallWidgetProps) {
  const title = getString(toolCall.result, "title");
  const category = getString(toolCall.result, "category");

  if (!title) {
    return <InlineIndicator icon={<Search size={11} style={{ color: colors.textMuted }} />} text="Loading proposal..." />;
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
      <Search size={11} style={{ color: colors.textMuted, flexShrink: 0 }} />
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: colors.textSecondary,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {title}
      </span>
      {category && <Badge variant="accent" compact>{category}</Badge>}
      <Badge variant="muted" compact>Loaded</Badge>
    </div>
  );
}

function GetSessionPlan({ toolCall, compact }: ToolCallWidgetProps) {
  // Result is the artifact or null
  const name = getString(toolCall.result, "name");
  const version = getNumber(toolCall.result, "version");

  if (!name) {
    return <InlineIndicator icon={<FileText size={11} style={{ color: colors.textMuted }} />} text="No plan artifact" />;
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
      <FileText size={11} style={{ color: colors.textMuted, flexShrink: 0 }} />
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: colors.textSecondary,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {name}
      </span>
      {version != null && <Badge variant="muted" compact>v{version}</Badge>}
      <Badge variant="muted" compact>Loaded</Badge>
    </div>
  );
}

function AnalyzeDependencies({ toolCall, compact }: ToolCallWidgetProps) {
  const result = toolCall.result;
  const totalProposals = getNumber(result, "total_proposals")
    ?? (result != null && typeof result === "object" && "summary" in (result as Record<string, unknown>)
      ? getNumber((result as Record<string, unknown>).summary, "total_proposals")
      : undefined);
  const hasCycles = result != null && typeof result === "object"
    && (result as Record<string, unknown>).has_cycles === true;

  if (totalProposals == null) {
    return <InlineIndicator icon={<GitBranch size={11} style={{ color: colors.textMuted }} />} text="Analyzing dependencies..." />;
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
      <GitBranch size={11} style={{ color: hasCycles ? colors.error : colors.success, flexShrink: 0 }} />
      <span
        style={{
          flex: 1,
          fontSize: compact ? 10.5 : 11,
          color: colors.textSecondary,
        }}
      >
        {totalProposals} proposals analyzed
      </span>
      {hasCycles && <Badge variant="error" compact>Cycles</Badge>}
      {!hasCycles && <Badge variant="success" compact>OK</Badge>}
    </div>
  );
}

// ============================================================================
// IdeationWidget (main component)
// ============================================================================

export const IdeationWidget = React.memo(function IdeationWidget(props: ToolCallWidgetProps) {
  const toolType = getToolType(props.toolCall.name);

  switch (toolType) {
    case "create_plan_artifact":
      return <PlanCreated {...props} />;
    case "update_plan_artifact":
      return <PlanUpdated {...props} />;
    case "link_proposals_to_plan":
      return <LinkProposals {...props} />;
    case "ask_user_question":
      return <AskUserQuestion {...props} />;
    case "list_session_proposals":
      return <ListProposals {...props} />;
    case "get_proposal":
      return <GetProposal {...props} />;
    case "get_session_plan":
      return <GetSessionPlan {...props} />;
    case "analyze_session_dependencies":
      return <AnalyzeDependencies {...props} />;
    default:
      return <InlineIndicator text={props.toolCall.name} />;
  }
});
