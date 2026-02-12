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

const TAURI_API_URL = process.env.TAURI_API_URL || "http://127.0.0.1:3847";

/**
 * MCP tool definition for permission handling
 * This tool is NOT scoped by agent type - it's always available
 */
export const permissionRequestTool: Tool = {
  name: "permission_request",
  description:
    "Internal tool for handling permission prompts from Claude CLI. This tool is called automatically when Claude needs permission for a non-pre-approved tool.",
  inputSchema: {
    type: "object",
    properties: {
      tool_name: {
        type: "string",
        description: "Name of the tool requesting permission",
      },
      tool_input: {
        type: "object",
        description: "Input arguments for the tool",
      },
      context: {
        type: "string",
        description: "Additional context about why the tool is being called",
      },
    },
    required: ["tool_name", "tool_input"],
  },
};

interface PermissionDecision {
  decision: "allow" | "deny";
  message?: string;
}

/** Normalize permission args from CLI (may send snake_case, camelCase, or name/input). */
function normalizePermissionArgs(
  args: Record<string, unknown>
): { tool_name: string; tool_input: Record<string, unknown>; context?: string } {
  const tool_name =
    (args.tool_name as string) ??
    (args.toolName as string) ??
    (args.name as string) ??
    "";
  const raw_input = args.tool_input ?? args.toolInput ?? args.input;
  const tool_input =
    raw_input != null && typeof raw_input === "object" && !Array.isArray(raw_input)
      ? (raw_input as Record<string, unknown>)
      : {};
  const context =
    (args.context as string) ?? (args.reason as string) ?? undefined;
  return { tool_name, tool_input, context };
}

/**
 * Handle a permission request by forwarding to Tauri backend
 * and waiting for user decision via long-poll.
 *
 * Flow:
 * 1. POST to /api/permission/request - registers request, emits Tauri event
 * 2. GET /api/permission/await/:id - blocks until user decides (5 min timeout)
 * 3. Return decision to Claude CLI
 *
 * @param args - Tool call details from Claude CLI (shape may vary)
 * @returns MCP tool result with decision (behavior + updatedInput / message)
 */
export async function handlePermissionRequest(
  args: Record<string, unknown>
): Promise<{ content: Array<{ type: "text"; text: string }> }> {
  const { tool_name, tool_input, context } = normalizePermissionArgs(args);

  if (!tool_name) {
    console.error("[RalphX MCP] Permission request missing tool name", args);
    return {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            behavior: "deny" as const,
            message: "Permission request missing tool name",
          }),
        },
      ],
    };
  }

  console.error(`[RalphX MCP] Permission request for tool: ${tool_name}`);

  // 1. Register permission request with Tauri backend
  let request_id: string;
  try {
    const body: { tool_name: string; tool_input: Record<string, unknown>; context?: string } = {
      tool_name,
      tool_input,
    };
    if (context !== undefined && context !== "") body.context = context;

    const registerResponse = await fetch(
      `${TAURI_API_URL}/api/permission/request`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      }
    );

    if (!registerResponse.ok) {
      throw new Error(
        `Failed to register permission request: ${registerResponse.statusText}`
      );
    }

    const result = (await registerResponse.json()) as { request_id: string };
    request_id = result.request_id;

    console.error(
      `[RalphX MCP] Permission request registered: ${request_id}`
    );
  } catch (error) {
    console.error(`[RalphX MCP] Failed to register permission request:`, error);
    return {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            behavior: "deny" as const,
            message: `Failed to register permission request: ${
              error instanceof Error ? error.message : String(error)
            }`,
          }),
        },
      ],
    };
  }

  // 2. Long-poll for user decision (5 minute timeout)
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), 5 * 60 * 1000);

  try {
    const decisionResponse = await fetch(
      `${TAURI_API_URL}/api/permission/await/${request_id}`,
      {
        method: "GET",
        signal: controller.signal,
      }
    );

    clearTimeout(timeoutId);

    if (!decisionResponse.ok) {
      if (decisionResponse.status === 408) {
        // Timeout - treat as deny
        console.error(
          `[RalphX MCP] Permission request ${request_id} timed out`
        );
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                behavior: "deny" as const,
                message:
                  "Permission request timed out waiting for user response",
              }),
            },
          ],
        };
      }
      throw new Error(`Permission decision error: ${decisionResponse.statusText}`);
    }

    const decision = (await decisionResponse.json()) as PermissionDecision;

    console.error(
      `[RalphX MCP] Permission ${decision.decision} for tool: ${tool_name}`
    );

    // Claude CLI expects permission-prompt-tool result to be a union:
    // - allow: { behavior: "allow", updatedInput: <record> }
    // - deny:  { behavior: "deny", message: <string> }
    const payload =
      decision.decision === "allow"
        ? { behavior: "allow" as const, updatedInput: tool_input }
        : {
            behavior: "deny" as const,
            message:
              decision.message ?? "User denied the tool call",
          };

    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(payload),
        },
      ],
    };
  } catch (error) {
    clearTimeout(timeoutId);
    if (error instanceof Error && error.name === "AbortError") {
      console.error(`[RalphX MCP] Permission request ${request_id} aborted`);
      return {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              behavior: "deny" as const,
              message: "Permission request timed out",
            }),
          },
        ],
      };
    }
    console.error(`[RalphX MCP] Permission request error:`, error);
    throw error;
  }
}
