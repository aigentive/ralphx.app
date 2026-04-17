import { safeError } from "./redact.js";

export function logToolsByAgent(toolsByAgent: Record<string, string[]>): void {
  console.error("\n=== RalphX MCP Server - All Available Tools ===\n");

  for (const [agentType, tools] of Object.entries(toolsByAgent)) {
    if (tools.length > 0) {
      safeError(`[${agentType}]`);
      tools.forEach((toolName) => safeError(`  - ${toolName}`));
      console.error("");
    }
  }

  console.error("=== End of Tools List ===\n");
}
