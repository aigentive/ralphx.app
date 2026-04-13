import { applyDelegationToolPolicy } from "./delegation-policy.js";
import { loadCanonicalMcpTools } from "./canonical-agent-metadata.js";
import { safeError } from "./redact.js";
import {
  ORCHESTRATOR_IDEATION,
  ORCHESTRATOR_IDEATION_READONLY,
  CHAT_TASK,
  CHAT_PROJECT,
  REVIEWER,
  REVIEW_CHAT,
  REVIEW_HISTORY,
  WORKER,
  CODER,
  SESSION_NAMER,
  MERGER,
  PROJECT_ANALYZER,
  SUPERVISOR,
  QA_PREP,
  QA_TESTER,
  ORCHESTRATOR,
  DEEP_RESEARCHER,
  MEMORY_MAINTAINER,
  MEMORY_CAPTURE,
  PLAN_CRITIC_COMPLETENESS,
  PLAN_CRITIC_IMPLEMENTATION_FEASIBILITY,
  PLAN_VERIFIER,
  IDEATION_TEAM_LEAD,
  IDEATION_TEAM_MEMBER,
  WORKER_TEAM_LEAD,
  WORKER_TEAM_MEMBER,
  IDEATION_SPECIALIST_BACKEND,
  IDEATION_SPECIALIST_FRONTEND,
  IDEATION_SPECIALIST_INFRA,
  IDEATION_SPECIALIST_UX,
  IDEATION_SPECIALIST_CODE_QUALITY,
  IDEATION_SPECIALIST_PROMPT_QUALITY,
  IDEATION_SPECIALIST_INTENT,
  IDEATION_SPECIALIST_PIPELINE_SAFETY,
  IDEATION_SPECIALIST_STATE_MACHINE,
  IDEATION_CRITIC,
  IDEATION_ADVOCATE,
} from "./agentNames.js";

/**
 * Tool scoping per agent type
 * Hard enforcement: each agent only sees tools appropriate for its role
 */
const TOOL_ALLOWLIST_BASE: Record<string, string[]> = {
  [ORCHESTRATOR_IDEATION]: [
    "create_task_proposal",
    "update_task_proposal",
    "archive_task_proposal",
    "delete_task_proposal",
    "finalize_proposals",
    "list_session_proposals",
    "get_proposal",
    "analyze_session_dependencies",
    "create_plan_artifact",
    "update_plan_artifact",
    "edit_plan_artifact",
    "get_artifact",
    "link_proposals_to_plan",
    "get_session_plan",
    "ask_user_question",
    "create_child_session",
    "get_parent_session_context",
    "delegate_start",
    "delegate_wait",
    "delegate_cancel",
    "get_session_messages",
    "get_team_artifacts",
    "update_plan_verification",
    "get_plan_verification",
    "revert_and_skip",
    "stop_verification",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
    "get_acceptance_status",
    "get_pending_confirmations",
    "get_verification_confirmation_status",
    "list_projects",
    "create_cross_project_session",
    "cross_project_guide",
    "migrate_proposals",
    "get_child_session_status",
    "send_ideation_session_message",
  ],
  [ORCHESTRATOR_IDEATION_READONLY]: [
    "list_session_proposals",
    "get_proposal",
    "get_artifact",
    "get_session_plan",
    "get_parent_session_context",
    "create_child_session",
    "delegate_start",
    "delegate_wait",
    "delegate_cancel",
    "get_plan_verification",
    "get_session_messages",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [CHAT_TASK]: [
    "update_task",
    "add_task_note",
    "get_task_details",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [CHAT_PROJECT]: [
    "suggest_task",
    "list_tasks",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
    "get_conversation_transcript",
  ],
  [REVIEWER]: [
    "complete_review",
    "create_followup_session",
    "delegate_start",
    "delegate_wait",
    "delegate_cancel",
    "get_task_issues",
    "get_step_progress",
    "get_issue_progress",
    "get_project_analysis",
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",
    "get_task_steps",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [REVIEW_CHAT]: [
    "approve_task",
    "request_task_changes",
    "get_review_notes",
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",
    "get_task_steps",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [REVIEW_HISTORY]: [
    "get_review_notes",
    "get_task_context",
    "get_task_issues",
    "get_task_steps",
    "get_step_progress",
    "get_issue_progress",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [WORKER]: [
    "start_step",
    "complete_step",
    "skip_step",
    "fail_step",
    "add_step",
    "get_step_progress",
    "get_step_context",
    "get_sub_steps",
    "execution_complete",
    "create_followup_session",
    "delegate_start",
    "delegate_wait",
    "delegate_cancel",
    "get_task_issues",
    "mark_issue_in_progress",
    "mark_issue_addressed",
    "get_project_analysis",
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",
    "get_task_steps",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [CODER]: [
    "start_step",
    "complete_step",
    "skip_step",
    "fail_step",
    "add_step",
    "get_step_progress",
    "get_step_context",
    "get_task_issues",
    "mark_issue_in_progress",
    "mark_issue_addressed",
    "get_project_analysis",
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",
    "get_task_steps",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [SESSION_NAMER]: ["update_session_title"],
  [MERGER]: [
    "report_conflict",
    "report_incomplete",
    "complete_merge",
    "get_merge_target",
    "delegate_start",
    "delegate_wait",
    "delegate_cancel",
    "get_project_analysis",
    "get_task_context",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [ORCHESTRATOR]: [
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [DEEP_RESEARCHER]: [
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [PROJECT_ANALYZER]: ["save_project_analysis", "get_project_analysis"],
  [SUPERVISOR]: [],
  [QA_PREP]: ["fs_read_file", "fs_list_dir", "fs_grep", "fs_glob"],
  [QA_TESTER]: [],
  [MEMORY_MAINTAINER]: [
    "upsert_memories",
    "mark_memory_obsolete",
    "refresh_memory_rule_index",
    "ingest_rule_file",
    "rebuild_archive_snapshots",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
    "get_conversation_transcript",
  ],
  [MEMORY_CAPTURE]: [
    "upsert_memories",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
    "get_conversation_transcript",
  ],
  [IDEATION_TEAM_LEAD]: [
    "request_team_plan",
    "request_teammate_spawn",
    "create_team_artifact",
    "get_team_artifacts",
    "get_team_session_state",
    "save_team_session_state",
    "create_task_proposal",
    "update_task_proposal",
    "archive_task_proposal",
    "delete_task_proposal",
    "finalize_proposals",
    "list_session_proposals",
    "get_proposal",
    "analyze_session_dependencies",
    "create_plan_artifact",
    "update_plan_artifact",
    "edit_plan_artifact",
    "get_artifact",
    "link_proposals_to_plan",
    "get_session_plan",
    "ask_user_question",
    "create_child_session",
    "get_parent_session_context",
    "get_session_messages",
    "update_plan_verification",
    "get_plan_verification",
    "revert_and_skip",
    "stop_verification",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
    "get_acceptance_status",
    "get_pending_confirmations",
    "get_verification_confirmation_status",
    "list_projects",
    "create_cross_project_session",
    "cross_project_guide",
    "migrate_proposals",
    "get_child_session_status",
    "send_ideation_session_message",
  ],
  [IDEATION_TEAM_MEMBER]: [
    "create_team_artifact",
    "get_team_artifacts",
    "get_session_plan",
    "list_session_proposals",
    "get_proposal",
    "get_artifact",
    "get_parent_session_context",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  ...((): Record<string, string[]> => {
    const IDEATION_SPECIALIST_RESEARCH_TOOLS = [
      "fs_read_file",
      "fs_list_dir",
      "fs_grep",
      "fs_glob",
      "create_team_artifact",
      "get_team_artifacts",
      "get_session_plan",
      "get_artifact",
      "list_session_proposals",
      "get_proposal",
      "get_parent_session_context",
      "search_memories",
      "get_memory",
      "get_memories_for_paths",
    ];
    const IDEATION_SPECIALIST_ENRICHMENT_TOOLS = [
      "fs_read_file",
      "fs_list_dir",
      "fs_grep",
      "fs_glob",
      "create_team_artifact",
      "get_team_artifacts",
      "get_session_plan",
      "get_artifact",
    ];
    return {
      [IDEATION_SPECIALIST_BACKEND]: IDEATION_SPECIALIST_RESEARCH_TOOLS,
      [IDEATION_SPECIALIST_FRONTEND]: IDEATION_SPECIALIST_RESEARCH_TOOLS,
      [IDEATION_SPECIALIST_INFRA]: IDEATION_SPECIALIST_RESEARCH_TOOLS,
      [IDEATION_SPECIALIST_UX]: IDEATION_SPECIALIST_RESEARCH_TOOLS,
      [IDEATION_SPECIALIST_CODE_QUALITY]: IDEATION_SPECIALIST_ENRICHMENT_TOOLS,
      [IDEATION_SPECIALIST_PROMPT_QUALITY]: IDEATION_SPECIALIST_ENRICHMENT_TOOLS,
      [IDEATION_SPECIALIST_INTENT]: [
        ...IDEATION_SPECIALIST_ENRICHMENT_TOOLS,
        "get_session_messages",
        "search_memories",
        "get_memory",
        "get_memories_for_paths",
      ],
      [IDEATION_SPECIALIST_PIPELINE_SAFETY]: IDEATION_SPECIALIST_ENRICHMENT_TOOLS,
      [IDEATION_SPECIALIST_STATE_MACHINE]: IDEATION_SPECIALIST_ENRICHMENT_TOOLS,
      [IDEATION_CRITIC]: IDEATION_SPECIALIST_RESEARCH_TOOLS,
      [IDEATION_ADVOCATE]: IDEATION_SPECIALIST_RESEARCH_TOOLS,
    };
  })(),
  [WORKER_TEAM_LEAD]: [
    "request_team_plan",
    "request_teammate_spawn",
    "create_team_artifact",
    "get_team_artifacts",
    "get_team_session_state",
    "save_team_session_state",
    "start_step",
    "complete_step",
    "skip_step",
    "fail_step",
    "add_step",
    "get_step_progress",
    "get_step_context",
    "get_sub_steps",
    "execution_complete",
    "get_task_issues",
    "mark_issue_in_progress",
    "mark_issue_addressed",
    "get_project_analysis",
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",
    "get_task_steps",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [WORKER_TEAM_MEMBER]: [
    "create_team_artifact",
    "get_team_artifacts",
    "start_step",
    "complete_step",
    "skip_step",
    "fail_step",
    "add_step",
    "get_step_progress",
    "get_step_context",
    "get_sub_steps",
    "get_task_issues",
    "mark_issue_in_progress",
    "mark_issue_addressed",
    "get_project_analysis",
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",
    "get_task_steps",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
  [PLAN_CRITIC_COMPLETENESS]: [
    "fs_read_file",
    "fs_list_dir",
    "fs_grep",
    "fs_glob",
    "get_session_plan",
    "get_artifact",
    "create_team_artifact",
  ],
  [PLAN_CRITIC_IMPLEMENTATION_FEASIBILITY]: [
    "fs_read_file",
    "fs_list_dir",
    "fs_grep",
    "fs_glob",
    "get_session_plan",
    "get_artifact",
    "create_team_artifact",
  ],
  [PLAN_VERIFIER]: [
    "fs_read_file",
    "fs_list_dir",
    "fs_grep",
    "fs_glob",
    "get_session_plan",
    "get_session_messages",
    "get_verification_round_artifacts",
    "get_parent_session_context",
    "delegate_start",
    "delegate_wait",
    "delegate_cancel",
    "report_verification_round",
    "complete_plan_verification",
    "get_plan_verification",
    "update_plan_artifact",
    "edit_plan_artifact",
    "send_ideation_session_message",
    "create_team_artifact",
    "list_session_proposals",
    "get_proposal",
    "search_memories",
    "get_memory",
    "get_memories_for_paths",
  ],
};

export const TOOL_ALLOWLIST = TOOL_ALLOWLIST_BASE;

let currentAgentType = "";

export function setAgentType(agentType: string): void {
  currentAgentType = agentType;
}

export function getAgentType(): string {
  return currentAgentType || process.env.RALPHX_AGENT_TYPE || "";
}

const TOOL_NAME_PATTERN = /^[a-z][a-z0-9_]*$/;

export function parseAllowedToolsFromArgs(knownToolNames: string[]): string[] | undefined {
  for (const arg of process.argv) {
    if (arg.startsWith("--allowed-tools=")) {
      const value = arg.substring("--allowed-tools=".length);
      if (!value) return undefined;
      if (value === "__NONE__") return [];
      const tools = value.split(",").map((t) => t.trim()).filter((t) => t.length > 0);
      const validated = tools.filter((t) => {
        if (!TOOL_NAME_PATTERN.test(t)) {
          safeError(`[RalphX MCP] WARN: Invalid tool name in --allowed-tools: "${t}" (skipped)`);
          return false;
        }
        return true;
      });
      const knownTools = new Set(knownToolNames);
      for (const t of validated) {
        if (!knownTools.has(t)) {
          safeError(`[RalphX MCP] WARN: --allowed-tools contains unknown tool "${t}" (not in ALL_TOOLS registry)`);
        }
      }
      return validated;
    }
  }
  return undefined;
}

export function getAllowedToolNames(knownToolNames: string[]): string[] {
  const agentType = getAgentType();

  const envAllowedTools = process.env.RALPHX_ALLOWED_MCP_TOOLS;
  if (envAllowedTools) {
    const tools = envAllowedTools.split(",").map((t) => t.trim()).filter((t) => t.length > 0);
    return applyDelegationToolPolicy(tools, agentType);
  }

  const cliTools = parseAllowedToolsFromArgs(knownToolNames);
  if (cliTools !== undefined) {
    return applyDelegationToolPolicy(cliTools, agentType);
  }

  const canonicalTools = loadCanonicalMcpTools(agentType);
  if (canonicalTools !== undefined) {
    console.error(
      `[RalphX MCP] WARN: --allowed-tools not provided, using canonical agent capabilities`
    );
    return applyDelegationToolPolicy(canonicalTools, agentType);
  }

  console.error(`[RalphX MCP] WARN: --allowed-tools not provided, using fallback TOOL_ALLOWLIST (may be stale)`);
  return applyDelegationToolPolicy(getToolsByAgent(knownToolNames)[agentType] || [], agentType);
}

export function getToolsByAgent(knownToolNames: string[]): Record<string, string[]> {
  return {
    ...TOOL_ALLOWLIST_BASE,
    debug: knownToolNames,
  };
}
