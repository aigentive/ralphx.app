/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */
import { Tool } from "@modelcontextprotocol/sdk/types.js";
export { TOOL_ALLOWLIST, LEGACY_TOOL_ALLOWLIST, setAgentType, getAgentType, } from "./tool-authorization.js";
/**
 * All available MCP tools
 * Tools are filtered based on RALPHX_AGENT_TYPE environment variable
 */
export declare const ALL_TOOLS: Tool[];
export declare function parseAllowedToolsFromArgs(): string[] | undefined;
export declare function getAllowedToolNames(): string[];
/**
 * Get filtered tools based on agent type
 * @returns Tools available to the current agent
 */
export declare function getFilteredTools(): Tool[];
/**
 * Check if a tool is allowed for the current agent type
 * @param toolName - Name of the tool to check
 * @returns true if allowed, false otherwise
 */
export declare function isToolAllowed(toolName: string): boolean;
/**
 * Get all tools regardless of agent type (for debugging)
 * @returns All available tools
 */
export declare function getAllTools(): Tool[];
/**
 * Get all tool names grouped by agent type (for debugging)
 * @returns Object mapping agent types to their allowed tools
 */
export declare function getToolsByAgent(): Record<string, string[]>;
/**
 * Return a compact repair hint for high-friction tools so weaker models can retry
 * with the expected payload shape instead of probing by trial and error.
 */
export declare function getToolRecoveryHint(toolName: string): string | null;
/**
 * Format a backend error message with an optional tool-specific usage hint.
 */
export declare function formatToolErrorMessage(toolName: string, message: string, details?: string): string;
/**
 * Print all available tools to stderr (for debugging)
 * Call this to see what tools the MCP server can provide
 */
export declare function logAllTools(): void;
//# sourceMappingURL=tools.d.ts.map