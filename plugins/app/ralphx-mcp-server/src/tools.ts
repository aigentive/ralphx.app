/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";
import { safeError } from "./redact.js";
import { PLAN_TOOLS } from "./plan-tools.js";
import { WORKER_CONTEXT_TOOLS } from "./worker-context-tools.js";
import { STEP_TOOLS } from "./step-tools.js";
import { ISSUE_TOOLS } from "./issue-tools.js";
import { FILESYSTEM_TOOLS } from "./filesystem-tools.js";
import { IDEATION_TOOLS } from "./ideation-tools.js";
import { WORKFLOW_TOOLS } from "./workflow-tools.js";
import { SUPPORT_TOOLS } from "./support-tools.js";
import {
  getAllowedToolNames as resolveAllowedToolNames,
  getToolsByAgent as resolveToolsByAgent,
  parseAllowedToolsFromArgs as parseAllowedToolsFromKnownRegistry,
} from "./tool-authorization.js";
export { TOOL_ALLOWLIST, setAgentType, getAgentType } from "./tool-authorization.js";

/**
 * All available MCP tools
 * Tools are filtered based on RALPHX_AGENT_TYPE environment variable
 */
export const ALL_TOOLS: Tool[] = [
  ...FILESYSTEM_TOOLS,
  ...IDEATION_TOOLS,

  ...WORKFLOW_TOOLS,
  // ========================================================================
  // PLAN ARTIFACT TOOLS (ralphx-ideation agent)
  // ========================================================================
  ...PLAN_TOOLS,

  // ========================================================================
  // WORKER CONTEXT TOOLS (worker agent)
  // ========================================================================
  ...WORKER_CONTEXT_TOOLS,

  // ========================================================================
  // STEP TOOLS (worker agent)
  // ========================================================================
  ...STEP_TOOLS,

  // ========================================================================
  // ISSUE TOOLS (worker + reviewer agents)
  // ========================================================================
  ...ISSUE_TOOLS,

  ...SUPPORT_TOOLS,
];

const ALL_TOOL_NAMES = ALL_TOOLS.map((tool) => tool.name);

export function parseAllowedToolsFromArgs(): string[] | undefined {
  return parseAllowedToolsFromKnownRegistry(ALL_TOOL_NAMES);
}

export function getAllowedToolNames(): string[] {
  return resolveAllowedToolNames(ALL_TOOL_NAMES);
}

/**
 * Get filtered tools based on agent type
 * @returns Tools available to the current agent
 */
export function getFilteredTools(): Tool[] {
  const allowedNames = getAllowedToolNames();
  return ALL_TOOLS.filter((tool) => allowedNames.includes(tool.name));
}

/**
 * Check if a tool is allowed for the current agent type
 * @param toolName - Name of the tool to check
 * @returns true if allowed, false otherwise
 */
export function isToolAllowed(toolName: string): boolean {
  const allowedNames = getAllowedToolNames();
  return allowedNames.includes(toolName);
}

/**
 * Get all tools regardless of agent type (for debugging)
 * @returns All available tools
 */
export function getAllTools(): Tool[] {
  return ALL_TOOLS;
}

/**
 * Get all tool names grouped by agent type (for debugging)
 * @returns Object mapping agent types to their allowed tools
 */
export function getToolsByAgent(): Record<string, string[]> {
  return resolveToolsByAgent(ALL_TOOL_NAMES);
}

function formatToolExamples(tool: Tool, limit = 1): string[] {
  const examples = ((tool.inputSchema as { examples?: unknown[] } | undefined)?.examples ?? [])
    .slice(0, limit)
    .map((example) => {
      try {
        return JSON.stringify(example);
      } catch {
        return String(example);
      }
    })
    .filter((example) => example.length > 0);

  return examples;
}

/**
 * Return a compact repair hint for high-friction tools so weaker models can retry
 * with the expected payload shape instead of probing by trial and error.
 */
export function getToolRecoveryHint(toolName: string): string | null {
  const tool = ALL_TOOLS.find((candidate) => candidate.name === toolName);
  if (!tool) {
    return null;
  }

  switch (toolName) {
    case "update_plan_verification": {
      const examples = formatToolExamples(tool, 2);
      return [
        "Use the PARENT ideation session_id as the canonical target. If a verification child session_id is passed, the backend remaps it automatically.",
        "If report_verification_round / complete_plan_verification are available, prefer those narrower helpers instead of this generic tool.",
        "Use status=reviewing with in_progress=true for mid-round updates; use verified or needs_revision with in_progress=false for terminal updates.",
        "Re-read get_plan_verification if generation/in_progress is unclear instead of guessing.",
        ...examples.map((example, index) =>
          index === 0
            ? `Example reviewing payload: ${example}`
            : `Example terminal payload: ${example}`
        ),
      ].join("\n");
    }
    case "report_verification_round": {
      const examples = formatToolExamples(tool);
      return [
        "Use this verifier-friendly helper for in-progress rounds on the PARENT ideation session.",
        "If a verification child session_id is passed, the backend remaps it to the parent automatically.",
        "You only provide round, gaps, and generation; status=reviewing and in_progress=true are filled in automatically.",
        ...examples.map((example) => `Example payload: ${example}`),
      ].join("\n");
    }
    case "complete_plan_verification": {
      const examples = formatToolExamples(tool, 2);
      return [
        "Use this verifier-friendly helper for terminal verification updates on the PARENT ideation session.",
        "If a verification child session_id is passed, the backend remaps it to the parent automatically.",
        "You provide the terminal status and generation; in_progress=false is filled in automatically.",
        "External sessions cannot use status=skipped.",
        ...examples.map((example, index) =>
          index === 0
            ? `Example terminal payload: ${example}`
            : `Example abort-cleanup payload: ${example}`
        ),
      ].join("\n");
    }
    case "get_plan_verification": {
      const examples = formatToolExamples(tool);
      return [
        "Call this on the PARENT ideation session before retrying report_verification_round, complete_plan_verification, or update_plan_verification. If a verification child session_id is passed, the backend remaps it to the parent automatically.",
        ...examples.map((example) => `Example payload: ${example}`),
      ].join("\n");
    }
    case "create_team_artifact": {
      const examples = formatToolExamples(tool);
      return [
        "Use the PARENT ideation session_id as the canonical target. If a verification child session id is passed, the backend remaps it to the parent automatically.",
        "For verifier critics, keep the exact artifact prefix and publish partial results instead of exploring further.",
        ...examples.map((example) => `Example payload: ${example}`),
      ].join("\n");
    }
    case "get_team_artifacts": {
      const examples = formatToolExamples(tool);
      return [
        "Read artifacts from the PARENT ideation session_id as the canonical target. If a verification child session id is passed, the backend remaps it to the parent automatically.",
        "Verification flows should usually prefer get_verification_round_artifacts instead of manually sorting summaries and then loading full artifact ids.",
        ...examples.map((example) => `Example payload: ${example}`),
      ].join("\n");
    }
    case "get_verification_round_artifacts": {
      const examples = formatToolExamples(tool);
      return [
        "Use this verifier helper instead of manually calling get_team_artifacts + get_artifact + client-side sorting for current-round artifacts.",
        "Provide the parent ideation session_id plus the title prefixes you expect; the MCP proxy filters by created_after and returns the latest match per prefix.",
        ...examples.map((example) => `Example payload: ${example}`),
      ].join("\n");
    }
    case "get_child_session_status": {
      const examples = formatToolExamples(tool);
      return [
        "When debugging a verification child, set include_recent_messages=true so you can inspect the last assistant/tool outputs.",
        ...examples.map((example) => `Example payload: ${example}`),
      ].join("\n");
    }
    case "send_ideation_session_message": {
      const examples = formatToolExamples(tool);
      return [
        "When nudging a verifier/critic, repeat full invariant context: SESSION_ID, ROUND, artifact prefix/schema, and explicit parent-session target.",
        ...examples.map((example) => `Example payload: ${example}`),
      ].join("\n");
    }
    default: {
      const examples = formatToolExamples(tool);
      if (examples.length === 0) {
        return null;
      }
      return examples.map((example) => `Example payload: ${example}`).join("\n");
    }
  }
}

/**
 * Format a backend error message with an optional tool-specific usage hint.
 */
export function formatToolErrorMessage(
  toolName: string,
  message: string,
  details?: string
): string {
  const repairHint = getToolRecoveryHint(toolName);
  return (
    `ERROR: ${message}` +
    (details ? `\n\nDetails: ${details}` : "") +
    (repairHint ? `\n\nUsage hint for ${toolName}:\n${repairHint}` : "")
  );
}

/**
 * Print all available tools to stderr (for debugging)
 * Call this to see what tools the MCP server can provide
 */
export function logAllTools(): void {
  console.error("\n=== RalphX MCP Server - All Available Tools ===\n");

  for (const [agentType, tools] of Object.entries(getToolsByAgent())) {
    if (tools.length > 0) {
      safeError(`[${agentType}]`);
      tools.forEach((t) => safeError(`  - ${t}`));
      console.error("");
    }
  }

  console.error("=== End of Tools List ===\n");
}
