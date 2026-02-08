/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */
import { Tool } from "@modelcontextprotocol/sdk/types.js";
/**
 * All available MCP tools
 * Tools are filtered based on RALPHX_AGENT_TYPE environment variable
 */
export declare const ALL_TOOLS: Tool[];
/**
 * Tool scoping per agent type
 * Hard enforcement: each agent only sees tools appropriate for its role
 */
export declare const TOOL_ALLOWLIST: Record<string, string[]>;
/**
 * Set the current agent type (called from index.ts after parsing CLI args)
 * @param agentType - The agent type to set
 */
export declare function setAgentType(agentType: string): void;
/**
 * Get the current agent type
 * @returns The current agent type
 */
export declare function getAgentType(): string;
/**
 * Get allowed tool names for the current agent type
 * @returns Array of tool names this agent is allowed to use
 */
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
 * Print all available tools to stderr (for debugging)
 * Call this to see what tools the MCP server can provide
 */
export declare function logAllTools(): void;
//# sourceMappingURL=tools.d.ts.map