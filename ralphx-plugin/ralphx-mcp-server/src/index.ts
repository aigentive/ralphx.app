#!/usr/bin/env node

/**
 * RalphX MCP Server
 *
 * A proxy MCP server that forwards tool calls to the RalphX Tauri backend via HTTP.
 * All business logic lives in Rust - this server is a thin transport layer.
 *
 * Tool scoping:
 * - Reads RALPHX_AGENT_TYPE from environment (set by Rust backend when spawning)
 * - Filters available tools based on agent type (hard enforcement)
 * - Each agent only sees tools appropriate for its role
 */

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";

import { callTauri, callTauriGet, TauriClientError } from "./tauri-client.js";
import {
  getFilteredTools,
  isToolAllowed,
  getAllowedToolNames,
  logAllTools,
  getToolsByAgent,
} from "./tools.js";
import {
  permissionRequestTool,
  handlePermissionRequest,
} from "./permission-handler.js";

// Agent type from environment (set by Rust backend when spawning Claude CLI)
const AGENT_TYPE = process.env.RALPHX_AGENT_TYPE || "unknown";

/**
 * Create and configure the MCP server
 */
const server = new Server(
  {
    name: "ralphx",
    version: "1.0.0",
  },
  {
    capabilities: {
      tools: {},
    },
  }
);

/**
 * List available tools (filtered by agent type)
 * Note: permission_request tool is always included (not scoped by agent type)
 */
server.setRequestHandler(ListToolsRequestSchema, async () => {
  const tools = getFilteredTools();

  // Always include permission_request tool (not scoped by agent type)
  const allTools = [...tools, permissionRequestTool];

  // Log tool scoping for debugging
  const allowedNames = getAllowedToolNames();
  console.error(
    `[RalphX MCP] Agent type: ${AGENT_TYPE}, Tools: ${allowedNames.length > 0 ? allowedNames.join(", ") : "none"} + permission_request`
  );

  return { tools: allTools };
});

/**
 * Execute tool calls (with authorization check)
 */
server.setRequestHandler(CallToolRequestSchema, async (request) => {
  const { name, arguments: args } = request.params;

  // Special handling for permission_request tool (always allowed, not scoped by agent type)
  if (name === "permission_request") {
    return handlePermissionRequest(
      args as Parameters<typeof handlePermissionRequest>[0]
    );
  }

  // Authorization check (defense in depth)
  if (!isToolAllowed(name)) {
    const allowedNames = getAllowedToolNames();
    const errorMessage =
      allowedNames.length > 0
        ? `Tool "${name}" is not available for agent type "${AGENT_TYPE}". Allowed tools: ${allowedNames.join(", ")}`
        : `Agent type "${AGENT_TYPE}" has no MCP tools available. This agent should use filesystem tools (Read, Grep, Glob, Bash, Edit, Write) instead.`;

    console.error(`[RalphX MCP] Unauthorized tool call: ${name}`);

    return {
      content: [
        {
          type: "text",
          text: `ERROR: ${errorMessage}`,
        },
      ],
      isError: true,
    };
  }

  try {
    // Forward to Tauri backend
    console.error(
      `[RalphX MCP] Calling Tauri: ${name} with args:`,
      JSON.stringify(args)
    );

    let result: unknown;

    // Special handling for GET endpoints with path parameters
    if (name === "get_task_context") {
      const { task_id } = args as { task_id: string };
      result = await callTauriGet(`task_context/${task_id}`);
    } else if (name === "get_artifact") {
      const { artifact_id } = args as { artifact_id: string };
      result = await callTauriGet(`artifact/${artifact_id}`);
    } else if (name === "get_artifact_version") {
      const { artifact_id, version } = args as {
        artifact_id: string;
        version: number;
      };
      result = await callTauriGet(`artifact/${artifact_id}/version/${version}`);
    } else if (name === "get_related_artifacts") {
      const { artifact_id } = args as { artifact_id: string };
      result = await callTauriGet(`artifact/${artifact_id}/related`);
    } else if (name === "get_plan_artifact") {
      // Also handle get_plan_artifact as GET
      const { artifact_id } = args as { artifact_id: string };
      result = await callTauriGet(`get_plan_artifact/${artifact_id}`);
    } else if (name === "get_session_plan") {
      // Also handle get_session_plan as GET
      const { session_id } = args as { session_id: string };
      result = await callTauriGet(`get_session_plan/${session_id}`);
    } else if (name === "get_task_steps") {
      // GET /api/task_steps/:task_id
      const { task_id } = args as { task_id: string };
      result = await callTauriGet(`task_steps/${task_id}`);
    } else if (name === "get_step_progress") {
      // GET /api/step_progress/:task_id
      const { task_id } = args as { task_id: string };
      result = await callTauriGet(`step_progress/${task_id}`);
    } else {
      // Default: POST request
      result = await callTauri(name, (args as Record<string, unknown>) || {});
    }

    console.error(`[RalphX MCP] Success: ${name}`);

    // Return result as JSON text
    return {
      content: [
        {
          type: "text",
          text: JSON.stringify(result, null, 2),
        },
      ],
    };
  } catch (error) {
    console.error(`[RalphX MCP] Error calling ${name}:`, error);

    if (error instanceof TauriClientError) {
      return {
        content: [
          {
            type: "text",
            text: `ERROR: ${error.message}${error.details ? `\n\nDetails: ${error.details}` : ""}`,
          },
        ],
        isError: true,
      };
    }

    return {
      content: [
        {
          type: "text",
          text: `ERROR: Unexpected error: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
      isError: true,
    };
  }
});

/**
 * Start the server
 */
async function main() {
  console.error("[RalphX MCP] Starting server...");
  console.error(`[RalphX MCP] Agent type: ${AGENT_TYPE}`);
  console.error(
    `[RalphX MCP] Tauri API URL: ${process.env.TAURI_API_URL || "http://127.0.0.1:3847"}`
  );

  // Log all tools if in debug mode or if RALPHX_DEBUG_TOOLS is set
  if (AGENT_TYPE === "debug" || process.env.RALPHX_DEBUG_TOOLS === "1") {
    logAllTools();
  }

  // Always log available tools for this agent
  const toolsByAgent = getToolsByAgent();
  const agentTools = toolsByAgent[AGENT_TYPE] || [];
  console.error(
    `[RalphX MCP] Tools for ${AGENT_TYPE}: ${agentTools.length > 0 ? agentTools.join(", ") : "(none - using filesystem tools)"}`
  );

  const transport = new StdioServerTransport();
  await server.connect(transport);

  console.error("[RalphX MCP] Server running on stdio");
}

main().catch((error) => {
  console.error("[RalphX MCP] Fatal error:", error);
  process.exit(1);
});
