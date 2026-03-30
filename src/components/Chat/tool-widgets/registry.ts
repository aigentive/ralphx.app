/**
 * Tool Call Widget Registry
 *
 * Maps tool names to specialized React widget components.
 * ToolCallIndicator checks this registry before falling back to the generic renderer.
 *
 * To register a new widget:
 *   1. Create src/components/Chat/tool-widgets/YourWidget.tsx implementing ToolCallWidgetProps
 *   2. Import and add to TOOL_CALL_WIDGETS below
 */

import type { ComponentType } from "react";
import type { ToolCallWidgetProps } from "./shared";
import { StepIndicator } from "./StepIndicator";
import { ContextWidget } from "./ContextWidget";
import { StepsManifestWidget } from "./StepsManifestWidget";
import { IssuesSummaryWidget } from "./IssuesSummaryWidget";
import { ArtifactWidget } from "./ArtifactWidget";
import { ReviewWidget } from "./ReviewWidget";
import { MergeWidget } from "./MergeWidget";
import { ProposalWidget } from "./ProposalWidget";
import { IdeationWidget } from "./IdeationWidget";
import { VerificationWidget } from "./VerificationWidget";
import { ChildSessionWidget } from "./ChildSessionWidget";
import { GrepWidget } from "./GrepWidget";
import { GlobWidget } from "./GlobWidget";
import { ReadWidget } from "./ReadWidget";
import { BashWidget } from "./BashWidget";
import { SkillWidget } from "./SkillWidget";
import { SendMessageWidget } from "./SendMessageWidget";
import { TaskCreateWidget, TaskUpdateWidget, TaskListWidget, TeamCreateWidget, TeamDeleteWidget } from "./TeamTaskWidgets";
import { SessionContextWidget, TeamSessionStateWidget, SearchMemoriesWidget, TeamPlanWidget } from "./McpContextWidgets";

/** Registry type: tool name (lowercase) → React component */
export type ToolCallWidgetRegistry = Record<string, ComponentType<ToolCallWidgetProps>>;

/**
 * The widget registry. Maps tool names to specialized widget components.
 * Tool names should be lowercase to match normalized lookup in ToolCallIndicator.
 */
export const TOOL_CALL_WIDGETS: ToolCallWidgetRegistry = {
  // Bash tool → BashWidget (terminal output card)
  "bash": BashWidget,
  // File read tool → ReadWidget (file preview card)
  "read": ReadWidget,
  // Search tools → GrepWidget / GlobWidget
  grep: GrepWidget,
  glob: GlobWidget,
  // Skill tool → SkillWidget (skill invocation card)
  "skill": SkillWidget,
  // Context tool → ContextWidget (always-visible context card)
  // Bare-name entries kept for backward compat with non-MCP contexts (test fixtures, CLI direct mode)
  "get_task_context": ContextWidget,
  // MCP-prefixed entries for actual MCP tool calls (getToolCallWidget uses exact-match lookup)
  "mcp__ralphx__get_task_context": ContextWidget,
  // Step lifecycle tools → StepIndicator (ultra-compact inline indicators)
  "mcp__ralphx__start_step": StepIndicator,
  "mcp__ralphx__complete_step": StepIndicator,
  "mcp__ralphx__add_step": StepIndicator,
  "mcp__ralphx__skip_step": StepIndicator,
  "mcp__ralphx__fail_step": StepIndicator,
  "mcp__ralphx__get_step_progress": StepIndicator,
  // Steps manifest → StepsManifestWidget (collapsible checklist)
  "get_task_steps": StepsManifestWidget,
  "mcp__ralphx__get_task_steps": StepsManifestWidget,
  // Issues summary → IssuesSummaryWidget (severity-badged issue list)
  "get_task_issues": IssuesSummaryWidget,
  "mcp__ralphx__get_task_issues": IssuesSummaryWidget,
  // Artifact tools → ArtifactWidget (type badge + title + markdown preview)
  "get_artifact": ArtifactWidget,
  "get_artifact_version": ArtifactWidget,
  "get_related_artifacts": ArtifactWidget,
  "search_project_artifacts": ArtifactWidget,
  "mcp__ralphx__get_artifact": ArtifactWidget,
  "mcp__ralphx__get_artifact_version": ArtifactWidget,
  "mcp__ralphx__get_related_artifacts": ArtifactWidget,
  "mcp__ralphx__search_project_artifacts": ArtifactWidget,
  // Review tools → ReviewWidget (outcome-colored cards + note list)
  "complete_review": ReviewWidget,
  "get_review_notes": ReviewWidget,
  "mcp__ralphx__complete_review": ReviewWidget,
  "mcp__ralphx__get_review_notes": ReviewWidget,
  // Merge tools → MergeWidget (success/conflict/incomplete cards + merge target)
  "mcp__ralphx__complete_merge": MergeWidget,
  "mcp__ralphx__report_conflict": MergeWidget,
  "mcp__ralphx__report_incomplete": MergeWidget,
  "mcp__ralphx__get_merge_target": MergeWidget,
  // Proposal CRUD tools → ProposalWidget
  "mcp__ralphx__create_task_proposal": ProposalWidget,
  "mcp__ralphx__update_task_proposal": ProposalWidget,
  "mcp__ralphx__delete_task_proposal": ProposalWidget,
  // Ideation session tools → IdeationWidget
  "mcp__ralphx__create_plan_artifact": IdeationWidget,
  "mcp__ralphx__update_plan_artifact": IdeationWidget,
  "mcp__ralphx__link_proposals_to_plan": IdeationWidget,
  "mcp__ralphx__ask_user_question": IdeationWidget,
  "mcp__ralphx__list_session_proposals": IdeationWidget,
  "mcp__ralphx__get_proposal": IdeationWidget,
  "mcp__ralphx__get_session_plan": IdeationWidget,
  "mcp__ralphx__analyze_session_dependencies": IdeationWidget,
  "mcp__ralphx__edit_plan_artifact": IdeationWidget,
  "mcp__ralphx__send_ideation_session_message": IdeationWidget,
  "mcp__ralphx__finalize_proposals": IdeationWidget,
  "mcp__ralphx__cross_project_guide": IdeationWidget,
  // Verification tools → VerificationWidget
  "mcp__ralphx__update_plan_verification": VerificationWidget,
  "mcp__ralphx__get_plan_verification": VerificationWidget,
  "mcp__ralphx__get_child_session_status": VerificationWidget,
  "mcp__ralphx__get_verification_confirmation_status": VerificationWidget,
  "mcp__ralphx__get_pending_confirmations": VerificationWidget,
  // Child session creation → ChildSessionWidget
  "mcp__ralphx__create_child_session": ChildSessionWidget,
  // SendMessage tool → SendMessageWidget (team message card)
  "sendmessage": SendMessageWidget,
  // Task management tools → TeamTaskWidgets
  "taskcreate": TaskCreateWidget,
  "taskupdate": TaskUpdateWidget,
  "tasklist": TaskListWidget,
  // Team lifecycle tools → TeamTaskWidgets
  "teamcreate": TeamCreateWidget,
  "teamdelete": TeamDeleteWidget,
  // MCP context/session/memory tools → McpContextWidgets
  "mcp__ralphx__get_parent_session_context": SessionContextWidget,
  "mcp__ralphx__get_team_session_state": TeamSessionStateWidget,
  "mcp__ralphx__search_memories": SearchMemoriesWidget,
  // MCP team plan tool → McpContextWidgets (WidgetCard with teammate list)
  "mcp__ralphx__request_team_plan": TeamPlanWidget,
};

/**
 * Look up a specialized widget for a tool name.
 * Returns undefined if no specialized widget is registered.
 */
export function getToolCallWidget(toolName: string): ComponentType<ToolCallWidgetProps> | undefined {
  return TOOL_CALL_WIDGETS[toolName.toLowerCase()];
}
