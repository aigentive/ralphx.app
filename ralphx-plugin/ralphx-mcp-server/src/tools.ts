/**
 * MCP tool definitions for RalphX
 * All tools are proxies that forward to Tauri backend via HTTP
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";
import { PLAN_TOOLS } from "./plan-tools.js";
import { WORKER_CONTEXT_TOOLS } from "./worker-context-tools.js";
import { STEP_TOOLS } from "./step-tools.js";

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
  {
    name: "apply_proposal_dependencies",
    description:
      "Apply AI-suggested dependencies directly to proposals. Clears existing dependencies and applies new ones. Used by dependency-suggester agent.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description: "The ideation session ID",
        },
        dependencies: {
          type: "array",
          items: {
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
              reason: {
                type: "string",
                description: "Brief explanation of why this dependency exists",
              },
            },
            required: ["proposal_id", "depends_on_id"],
          },
          description: "Array of dependency suggestions to apply",
        },
      },
      required: ["session_id", "dependencies"],
    },
  },
  {
    name: "update_session_title",
    description:
      "Update the title of an ideation session. Used by session-namer agent to set auto-generated titles.",
    inputSchema: {
      type: "object",
      properties: {
        session_id: {
          type: "string",
          description: "The ideation session ID to update",
        },
        title: {
          type: "string",
          description: "The new title for the session (exactly 2 words)",
        },
      },
      required: ["session_id", "title"],
    },
  },
  {
    name: "list_session_proposals",
    description:
      "List all task proposals in an ideation session. Returns summary info (id, title, category, priority, dependencies). Use get_proposal for full details including steps and acceptance criteria.",
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
  {
    name: "get_proposal",
    description:
      "Get full details of a task proposal including steps and acceptance criteria. Use after list_session_proposals to get complete information for a specific proposal.",
    inputSchema: {
      type: "object",
      properties: {
        proposal_id: {
          type: "string",
          description: "The proposal ID to fetch",
        },
      },
      required: ["proposal_id"],
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
  {
    name: "get_review_notes",
    description:
      "Get all review feedback for a task. Call this before re-executing a task to understand what needs to be fixed.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID to get review notes for",
        },
      },
      required: ["task_id"],
    },
  },
  {
    name: "approve_task",
    description:
      "Approve a task after AI review. ONLY available when task is in 'review_passed' status (awaiting human decision). " +
      "Use this when the user confirms they want to approve the task after discussing the review with you. " +
      "This will NOT work during active review - use complete_review for that.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID to approve",
        },
        comment: {
          type: "string",
          description: "Optional approval comment or notes",
        },
      },
      required: ["task_id"],
    },
  },
  {
    name: "request_task_changes",
    description:
      "Request changes on a task after AI review. ONLY available when task is in 'review_passed' status (awaiting human decision). " +
      "Use this when the user wants to request changes after discussing the review with you. " +
      "This will NOT work during active review - use complete_review for that.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID to request changes on",
        },
        feedback: {
          type: "string",
          description: "Detailed feedback explaining what changes are needed",
        },
      },
      required: ["task_id", "feedback"],
    },
  },

  // ========================================================================
  // PLAN ARTIFACT TOOLS (orchestrator-ideation agent)
  // ========================================================================
  ...PLAN_TOOLS,

  // ========================================================================
  // WORKER CONTEXT TOOLS (worker agent)
  // ========================================================================
  ...WORKER_CONTEXT_TOOLS,

  // ========================================================================
  // STEP TOOLS (worker agent)
  // ========================================================================
  ...STEP_TOOLS,
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
    // Note: add_proposal_dependency removed - dependencies are now auto-suggested by dependency-suggester agent
    "list_session_proposals",
    "get_proposal",
    "create_plan_artifact",
    "update_plan_artifact",
    "get_plan_artifact",
    "link_proposals_to_plan",
    "get_session_plan",
  ],
  "chat-task": ["update_task", "add_task_note", "get_task_details"],
  "chat-project": ["suggest_task", "list_tasks"],
  "ralphx-reviewer": ["complete_review"],
  // Post-review chat agent - helps user discuss review findings and take action
  "ralphx-review-chat": ["get_review_notes", "approve_task", "request_task_changes"],
  "ralphx-worker": [
    "get_task_context",
    "get_artifact",
    "get_artifact_version",
    "get_related_artifacts",
    "search_project_artifacts",
    "get_review_notes",
    "get_task_steps",
    "start_step",
    "complete_step",
    "skip_step",
    "fail_step",
    "add_step",
    "get_step_progress",
  ],
  // Session naming agent - generates titles for IDA sessions
  "session-namer": ["update_session_title"],
  // Dependency suggester agent - analyzes proposals and auto-applies dependencies
  "dependency-suggester": ["apply_proposal_dependencies"],
  // These agents have NO MCP tools - they use filesystem tools only
  supervisor: [],
  "qa-prep": [],
  "qa-tester": [],
  // Debug mode: shows ALL tools (use RALPHX_AGENT_TYPE=debug)
  debug: ALL_TOOLS.map((t) => t.name),
};

/**
 * Module-level agent type storage
 * Set by index.ts on startup after parsing CLI args
 * This is needed because CLI args take precedence over env vars
 * (Claude CLI doesn't pass env vars to MCP servers it spawns)
 */
let currentAgentType = "";

/**
 * Set the current agent type (called from index.ts after parsing CLI args)
 * @param agentType - The agent type to set
 */
export function setAgentType(agentType: string): void {
  currentAgentType = agentType;
}

/**
 * Get the current agent type
 * @returns The current agent type
 */
export function getAgentType(): string {
  return currentAgentType || process.env.RALPHX_AGENT_TYPE || "";
}

/**
 * Get allowed tool names for the current agent type
 * @returns Array of tool names this agent is allowed to use
 */
export function getAllowedToolNames(): string[] {
  const agentType = getAgentType();
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

/**
 * Get all tools regardless of agent type (for debugging)
 * @returns All available tools
 */
export function getAllTools(): Tool[] {
  return ALL_TOOLS;
}

/**
 * Get all tool names grouped by agent type (for debugging)
 * @returns Object mapping agent types to their allowed tools
 */
export function getToolsByAgent(): Record<string, string[]> {
  return TOOL_ALLOWLIST;
}

/**
 * Print all available tools to stderr (for debugging)
 * Call this to see what tools the MCP server can provide
 */
export function logAllTools(): void {
  console.error("\n=== RalphX MCP Server - All Available Tools ===\n");

  for (const [agentType, tools] of Object.entries(TOOL_ALLOWLIST)) {
    if (tools.length > 0) {
      console.error(`[${agentType}]`);
      tools.forEach((t) => console.error(`  - ${t}`));
      console.error("");
    }
  }

  console.error("=== End of Tools List ===\n");
}
