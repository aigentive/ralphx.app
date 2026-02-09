/**
 * MCP tool definitions for plan artifact management
 * Used by orchestrator-ideation agent to create and manage implementation plans
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";

/**
 * Plan artifact tools for orchestrator-ideation agent
 * All tools are proxies that forward to Tauri backend via HTTP
 */
export const PLAN_TOOLS: Tool[] = [
  {
    name: "create_plan_artifact",
    description:
      "Create a new implementation plan artifact linked to the ideation session. Use this when the user describes a complex feature that needs architectural planning before breaking into tasks. The plan is stored as a Specification artifact and can be referenced by task proposals.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description: "The ideation session ID (provided in context)",
        },
        title: {
          type: "string",
          description:
            "Plan title (e.g., 'Real-time Collaboration Implementation Plan')",
        },
        content: {
          type: "string",
          description:
            "Plan content in markdown format. Should include architecture decisions, data flow, key implementation details, and considerations.",
        },
      },
      required: ["session_id", "title", "content"],
    },
  },
  {
    name: "update_plan_artifact",
    description:
      "Update an existing implementation plan's content. Creates a NEW version with a new artifact ID (immutable version chain). Stale artifact IDs are auto-resolved: you can pass any previous version's ID and it will resolve to the latest before updating. Linked proposals are automatically re-linked to the new version (plan_version_at_creation is preserved). The response includes `previous_artifact_id` and `session_id` for reference. You do NOT need to call get_session_plan between updates to refresh the ID.",
    inputSchema: {
      type: "object",
      properties: {
        artifact_id: {
          type: "string",
          description:
            "The artifact ID of the plan to update. Can be any version ID — stale IDs are auto-resolved to the latest version.",
        },
        content: {
          type: "string",
          description:
            "Updated plan content in markdown format. This will create a new version of the artifact with a new ID.",
        },
      },
      required: ["artifact_id", "content"],
    },
  },
  {
    name: "get_plan_artifact",
    description:
      "Retrieve a plan artifact's content by version ID. Returns the content of the specific version requested (not necessarily the latest). Use get_session_plan to find the current latest version for a session. Note: unlike update_plan_artifact and link_proposals_to_plan, this does NOT auto-resolve stale IDs.",
    inputSchema: {
      type: "object",
      properties: {
        artifact_id: {
          type: "string",
          description:
            "The artifact ID of the specific version to retrieve. Returns that version's content, not the latest.",
        },
      },
      required: ["artifact_id"],
    },
  },
  {
    name: "link_proposals_to_plan",
    description:
      "Link multiple task proposals to an implementation plan. Use after creating proposals to establish the connection between the plan and its derived tasks. Stale artifact IDs are auto-resolved: you can pass any previous version's ID and it will resolve to the latest before linking. This enables traceability and allows the system to suggest updates when the plan changes.",
    inputSchema: {
      type: "object",
      properties: {
        proposal_ids: {
          type: "array",
          items: { type: "string" },
          description: "Array of proposal IDs to link to the plan",
        },
        artifact_id: {
          type: "string",
          description:
            "The plan artifact ID to link proposals to. Can be any version ID — stale IDs are auto-resolved to the latest version.",
        },
      },
      required: ["proposal_ids", "artifact_id"],
    },
  },
  {
    name: "get_session_plan",
    description:
      "Get the implementation plan artifact for the current ideation session, if one exists. Use to check if a plan has already been created before suggesting a new one.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description: "The ideation session ID",
        },
      },
      required: ["session_id"],
    },
  },
];
