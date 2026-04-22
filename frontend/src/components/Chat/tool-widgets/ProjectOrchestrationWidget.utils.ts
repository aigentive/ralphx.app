import {
  getString,
  parseMcpToolResult,
  type ToolCall,
} from "./shared.constants";

const PROJECT_ORCHESTRATION_TOOLS = new Set([
  "v1_get_agent_guide",
  "v1_list_ideation_sessions",
  "v1_get_project_status",
  "v1_get_ideation_status",
  "v1_get_ideation_messages",
  "v1_get_plan",
  "v1_get_plan_verification",
  "v1_list_proposals",
  "v1_get_session_tasks",
  "v1_send_ideation_message",
]);

export function canonicalProjectToolName(toolName: string): string {
  const normalized = toolName.trim().toLowerCase();
  if (normalized.startsWith("mcp__ralphx__")) {
    return normalized.slice("mcp__ralphx__".length);
  }
  if (normalized.startsWith("ralphx::")) {
    return normalized.slice("ralphx::".length);
  }
  if (normalized.startsWith("ralphx:")) {
    return normalized.slice("ralphx:".length);
  }
  return normalized;
}

export function projectIdeationSessionId(toolCall: ToolCall): string | undefined {
  const parsed = parseMcpToolResult(toolCall.result);
  return (
    getString(parsed, "sessionId") ??
    getString(parsed, "session_id") ??
    getString(toolCall.arguments, "sessionId") ??
    getString(toolCall.arguments, "session_id")
  );
}

export function shouldHideCompletedProjectOrchestrationToolCall(toolCall: ToolCall): boolean {
  const canonicalName = canonicalProjectToolName(toolCall.name);
  if (!PROJECT_ORCHESTRATION_TOOLS.has(canonicalName)) {
    return false;
  }
  if (toolCall.error || toolCall.result == null) {
    return false;
  }
  if (canonicalName === "v1_send_ideation_message" && projectIdeationSessionId(toolCall)) {
    return false;
  }
  return true;
}
