/**
 * Permission handler for UI-based approval of tool calls
 *
 * This MCP tool is called by Claude CLI when it needs permission for a tool
 * that wasn't pre-approved via --allowedTools. It:
 * 1. Forwards the permission request to the Tauri backend
 * 2. Long-polls for user decision (5 minute timeout)
 * 3. Returns the decision to Claude CLI
 *
 * The Tauri backend emits a Tauri event that triggers the PermissionDialog
 * in the frontend, allowing the user to approve or deny the tool call.
 */
import { Tool } from "@modelcontextprotocol/sdk/types.js";
/**
 * MCP tool definition for permission handling
 * This tool is NOT scoped by agent type - it's always available
 */
export declare const permissionRequestTool: Tool;
/**
 * Handle a permission request by forwarding to Tauri backend
 * and waiting for user decision via long-poll.
 *
 * Flow:
 * 1. POST to /api/permission/request - registers request, emits Tauri event
 * 2. GET /api/permission/await/:id - blocks until user decides (5 min timeout)
 * 3. Return decision to Claude CLI
 *
 * @param args - Tool call details from Claude CLI
 * @returns MCP tool result with decision (allowed: true/false)
 */
export declare function handlePermissionRequest(args: {
    tool_name: string;
    tool_input: Record<string, unknown>;
    context?: string;
}): Promise<{
    content: Array<{
        type: "text";
        text: string;
    }>;
}>;
//# sourceMappingURL=permission-handler.d.ts.map