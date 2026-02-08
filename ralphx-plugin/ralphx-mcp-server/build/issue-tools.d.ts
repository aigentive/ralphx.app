/**
 * MCP tool definitions for review issue management
 * Used by worker agent (during re-execution) and reviewer agent (during re-reviews)
 */
import { Tool } from "@modelcontextprotocol/sdk/types.js";
/**
 * Issue tools for worker and reviewer agents
 * All tools are proxies that forward to Tauri backend via HTTP
 */
export declare const ISSUE_TOOLS: Tool[];
//# sourceMappingURL=issue-tools.d.ts.map