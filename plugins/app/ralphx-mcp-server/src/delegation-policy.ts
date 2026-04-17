import { loadCanonicalAgentDefinition } from "./canonical-agent-metadata.js";

const DELEGATION_TOOL_NAMES = new Set([
  "delegate_start",
  "delegate_wait",
  "delegate_cancel",
]);
function agentCanDelegate(agentType: string): boolean {
  const definition = loadCanonicalAgentDefinition(agentType);
  return Boolean(definition?.delegation?.allowed_targets?.length);
}

export function applyDelegationToolPolicy(toolNames: string[], agentType: string): string[] {
  if (agentCanDelegate(agentType)) {
    return toolNames;
  }
  return toolNames.filter((toolName) => !DELEGATION_TOOL_NAMES.has(toolName));
}
