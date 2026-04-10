const MCP_RALPHX_PREFIX = "mcp__ralphx__";
const RALPHX_SERVER_PREFIX = "ralphx:";

export function canonicalizeToolName(toolName: string): string {
  const normalized = toolName.trim().toLowerCase();

  if (normalized.startsWith(MCP_RALPHX_PREFIX)) {
    return normalized.slice(MCP_RALPHX_PREFIX.length);
  }

  if (normalized.startsWith(RALPHX_SERVER_PREFIX)) {
    return normalized.slice(RALPHX_SERVER_PREFIX.length);
  }

  return normalized;
}

export function getToolCallLookupCandidates(toolName: string): string[] {
  const normalized = toolName.trim().toLowerCase();
  const canonical = canonicalizeToolName(normalized);
  const candidates = [normalized];

  if (!candidates.includes(canonical)) {
    candidates.push(canonical);
  }

  const mcpCandidate = `${MCP_RALPHX_PREFIX}${canonical}`;
  if (!candidates.includes(mcpCandidate)) {
    candidates.push(mcpCandidate);
  }

  return candidates;
}
