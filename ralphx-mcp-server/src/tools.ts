/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";
import { PLAN_TOOLS } from "./plan-tools.js";

/**
 * All available MCP tools
 * Tools are filtered based on RALPHX_AGENT_TYPE environment variable
 */
export const ALL_TOOLS: Tool[] = [
  // ========================================================================
  // IDEATION TOOLS (orchestrator-ideation agent)
  // ========================================================================
  {
    name: "create_task_proposal",
    description:
      "Create a new task proposal in the ideation session. Use this when the user describes a new feature, fix, or improvement they want to implement.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description: "The ideation session ID (provided in context)",
        },
        title: {
          type: "string",
          description: "Clear, concise task title (e.g., 'Add dark mode toggle')",
        },
        description: {
          type: "string",
          description: "Detailed description of what needs to be done",
        },
        category: {
          type: "string",
          enum: ["feature", "fix", "setup", "testing", "refactor", "docs"],
          description: "Task category",
        },
        priority: {
          type: "string",
          enum: ["critical", "high", "medium", "low"],
          description: "Task priority level",
        },
        steps: {
          type: "array",
          items: { type: "string" },
          description: "Step-by-step implementation plan",
        },
        acceptance_criteria: {
          type: "array",
          items: { type: "string" },
          description: "Criteria to verify task completion",
        },
      },
      required: ["session_id", "title", "category"],
    },
  },
  {
    name: "update_task_proposal",
    description:
      "Update an existing task proposal. Use when the user wants to modify a proposal's details, priority, or implementation plan.",
    inputSchema: {
      type: "object",
      properties: {
        proposal_id: {
          type: "string",
          description: "The proposal ID to update",
        },
        title: {
          type: "string",
          description: "Updated task title",
        },
        description: {
          type: "string",
          description: "Updated description",
        },
        category: {
          type: "string",
          enum: ["feature", "fix", "setup", "testing", "refactor", "docs"],
          description: "Updated category",
        },
        priority: {
          type: "string",
          enum: ["critical", "high", "medium", "low"],
          description: "Updated priority",
        },
        steps: {
          type: "array",
          items: { type: "string" },
          description: "Updated implementation steps",
        },
        acceptance_criteria: {
          type: "array",
          items: { type: "string" },
          description: "Updated acceptance criteria",
        },
      },
      required: ["proposal_id"],
    },
  },
  {
    name: "delete_task_proposal",
    description:
      "Delete a task proposal. Use when the user wants to remove a proposal that's no longer needed.",
    inputSchema: {
      type: "object",
      properties: {
        proposal_id: {
          type: "string",
          description: "The proposal ID to delete",
        },
      },
      required: ["proposal_id"],
    },
  },
  {
    name: "add_proposal_dependency",
    description:
      "Add a dependency relationship between two proposals. Use when one task must be completed before another can start.",
    inputSchema: {
      type: "object",
      properties: {
        proposal_id: {
          type: "string",
          description: "The proposal that depends on another",
        },
        depends_on_id: {
          type: "string",
          description: "The proposal that must be completed first",
        },
      },
      required: ["proposal_id", "depends_on_id"],
    },
  },

  // ========================================================================
  // TASK TOOLS (chat-task agent)
  // ========================================================================
  {
    name: "update_task",
    description:
      "Update an existing task's details. Use when the user wants to modify task title, description, priority, or status.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID to update",
        },
        title: {
          type: "string",
          description: "Updated task title",
        },
        description: {
          type: "string",
          description: "Updated description",
        },
        priority: {
          type: "string",
          enum: ["critical", "high", "medium", "low"],
          description: "Updated priority",
        },
        status: {
          type: "string",
          enum: [
            "backlog",
            "ready",
            "in_progress",
            "blocked",
            "review",
            "done",
            "cancelled",
          ],
          description: "Updated status",
        },
      },
      required: ["task_id"],
    },
  },
  {
    name: "add_task_note",
    description:
      "Add a note or comment to a task. Use when the user wants to document progress, issues, or decisions.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID",
        },
        note: {
          type: "string",
          description: "The note content",
        },
      },
      required: ["task_id", "note"],
    },
  },
  {
    name: "get_task_details",
    description:
      "Get full details for a task including current status, notes, and history. Use when you need complete task information.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID",
        },
      },
      required: ["task_id"],
    },
  },

  // ========================================================================
  // PROJECT TOOLS (chat-project agent)
  // ========================================================================
  {
    name: "suggest_task",
    description:
      "Suggest a new task based on project analysis. Use when you've identified something that should be done based on codebase exploration.",
    inputSchema: {
      type: "object",
      properties: {
        project_id: {
          type: "string",
          description: "The project ID (provided in context)",
        },
        title: {
          type: "string",
          description: "Suggested task title",
        },
        description: {
          type: "string",
          description: "Why this task should be done",
        },
        category: {
          type: "string",
          enum: ["feature", "fix", "setup", "testing", "refactor", "docs"],
          description: "Task category",
        },
        priority: {
          type: "string",
          enum: ["critical", "high", "medium", "low"],
          description: "Suggested priority",
        },
      },
      required: ["project_id", "title", "description", "category"],
    },
  },
  {
    name: "list_tasks",
    description:
      "List tasks in the project with optional filtering. Use to answer questions about what tasks exist, their status, or priorities.",
    inputSchema: {
      type: "object",
      properties: {
        project_id: {
          type: "string",
          description: "The project ID",
        },
        status: {
          type: "string",
          enum: [
            "backlog",
            "ready",
            "in_progress",
            "blocked",
            "review",
            "done",
            "cancelled",
          ],
          description: "Filter by status (optional)",
        },
        category: {
          type: "string",
          enum: ["feature", "fix", "setup", "testing", "refactor", "docs"],
          description: "Filter by category (optional)",
        },
      },
      required: ["project_id"],
    },
  },

  // ========================================================================
  // REVIEW TOOLS (reviewer agent)
  // ========================================================================
  {
    name: "complete_review",
    description:
      "Submit a code review decision. Use after reviewing changes to approve, request changes, or escalate to supervisor.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task being reviewed",
        },
        decision: {
          type: "string",
          enum: ["approved", "needs_changes", "escalate"],
          description:
            "Review decision: approved (ship it), needs_changes (fixable issues), escalate (major concerns)",
        },
        feedback: {
          type: "string",
          description:
            "Detailed feedback: what's good, what needs improvement, specific issues found",
        },
        issues: {
          type: "array",
          items: {
            type: "object",
            properties: {
              severity: {
                type: "string",
                enum: ["critical", "major", "minor", "suggestion"],
              },
              file: { type: "string" },
              line: { type: "number" },
              description: { type: "string" },
            },
            required: ["severity", "description"],
          },
          description: "Specific issues found during review",
        },
      },
      required: ["task_id", "decision", "feedback"],
    },
  },

  // ========================================================================
  // PLAN ARTIFACT TOOLS (orchestrator-ideation agent)
  // ========================================================================
  ...PLAN_TOOLS,
];

/**
 * Tool scoping per agent type
 * Hard enforcement: each agent only sees tools appropriate for its role
 */
export const TOOL_ALLOWLIST: Record<string, string[]> = {
  "orchestrator-ideation": [
    "create_task_proposal",
    "update_task_proposal",
    "delete_task_proposal",
    "add_proposal_dependency",
    "create_plan_artifact",
    "update_plan_artifact",
    "get_plan_artifact",
    "link_proposals_to_plan",
    "get_session_plan",
  ],
  "chat-task": ["update_task", "add_task_note", "get_task_details"],
  "chat-project": ["suggest_task", "list_tasks"],
  "reviewer": ["complete_review"],
  // These agents have NO MCP tools - they use filesystem tools only
  worker: [],
  supervisor: [],
  "qa-prep": [],
  "qa-tester": [],
};

/**
 * Get allowed tool names for the current agent type
 * @returns Array of tool names this agent is allowed to use
 */
export function getAllowedToolNames(): string[] {
  const agentType = process.env.RALPHX_AGENT_TYPE || "";
  return TOOL_ALLOWLIST[agentType] || [];
}

/**
 * Get filtered tools based on agent type
 * @returns Tools available to the current agent
 */
export function getFilteredTools(): Tool[] {
  const allowedNames = getAllowedToolNames();
  return ALL_TOOLS.filter((tool) => allowedNames.includes(tool.name));
}

/**
 * Check if a tool is allowed for the current agent type
 * @param toolName - Name of the tool to check
 * @returns true if allowed, false otherwise
 */
export function isToolAllowed(toolName: string): boolean {
  const allowedNames = getAllowedToolNames();
  return allowedNames.includes(toolName);
}
