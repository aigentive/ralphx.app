const MCP_RALPHX_PREFIX = "mcp__ralphx__";
const RALPHX_SERVER_PREFIX = "ralphx:";
const RALPHX_DOUBLE_COLON_PREFIX = "ralphx::";

export function canonicalizeToolName(toolName: string): string {
  const normalized = toolName.trim().toLowerCase();

  if (normalized.startsWith(MCP_RALPHX_PREFIX)) {
    return normalized.slice(MCP_RALPHX_PREFIX.length);
  }

  if (normalized.startsWith(RALPHX_DOUBLE_COLON_PREFIX)) {
    return normalized.slice(RALPHX_DOUBLE_COLON_PREFIX.length);
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

  const serverCandidate = `${RALPHX_SERVER_PREFIX}${canonical}`;
  if (!candidates.includes(serverCandidate)) {
    candidates.push(serverCandidate);
  }

  const doubleColonCandidate = `${RALPHX_DOUBLE_COLON_PREFIX}${canonical}`;
  if (!candidates.includes(doubleColonCandidate)) {
    candidates.push(doubleColonCandidate);
  }

  return candidates;
}
