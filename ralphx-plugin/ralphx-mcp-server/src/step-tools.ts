/**
 * MCP tool definitions for task step management
 * Used by worker agent to track progress during execution
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";

/**
 * Step tools for worker agent
 * All tools are proxies that forward to Tauri backend via HTTP
 */
export const STEP_TOOLS: Tool[] = [
  {
    name: "get_task_steps",
    description:
      "Fetch all steps for a task. Call this at the start of task execution to see the implementation plan. Returns steps ordered by sort_order.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The ID of the task to get steps for",
        },
      },
      required: ["task_id"],
    },
  },
  {
    name: "start_step",
    description:
      "Mark a step as in-progress. Call this BEFORE starting work on a step to track progress. Only pending steps can be started.",
    inputSchema: {
      type: "object",
      properties: {
        step_id: {
          type: "string",
          description: "The ID of the step to start",
        },
      },
      required: ["step_id"],
    },
  },
  {
    name: "complete_step",
    description:
      "Mark a step as completed. Call this AFTER finishing a step successfully. Only in-progress steps can be completed. Optionally include a completion note describing what was done.",
    inputSchema: {
      type: "object",
      properties: {
        step_id: {
          type: "string",
          description: "The ID of the step to complete",
        },
        note: {
          type: "string",
          description:
            "Optional note describing what was done or any relevant details",
        },
      },
      required: ["step_id"],
    },
  },
  {
    name: "skip_step",
    description:
      "Mark a step as skipped. Use when a step is not applicable or not needed for this task. Provide a reason explaining why the step was skipped.",
    inputSchema: {
      type: "object",
      properties: {
        step_id: {
          type: "string",
          description: "The ID of the step to skip",
        },
        reason: {
          type: "string",
          description: "Explanation of why this step is being skipped",
        },
      },
      required: ["step_id", "reason"],
    },
  },
  {
    name: "fail_step",
    description:
      "Mark a step as failed. Use when a step encounters an error or cannot be completed. Only in-progress steps can be marked as failed. Provide details about the error.",
    inputSchema: {
      type: "object",
      properties: {
        step_id: {
          type: "string",
          description: "The ID of the step that failed",
        },
        error: {
          type: "string",
          description: "Description of the error or why the step failed",
        },
      },
      required: ["step_id", "error"],
    },
  },
  {
    name: "add_step",
    description:
      "Add a new step to the task during execution. Use when you realize additional work is needed that wasn't in the original plan. The step will be inserted at the appropriate position.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID to add the step to",
        },
        title: {
          type: "string",
          description: "Brief title describing what this step does",
        },
        description: {
          type: "string",
          description: "Optional detailed description of the step",
        },
        after_step_id: {
          type: "string",
          description:
            "Optional: ID of the step to insert after. If not provided, adds to the end.",
        },
      },
      required: ["task_id", "title"],
    },
  },
  {
    name: "get_step_progress",
    description:
      "Get a summary of step progress for a task including counts by status, current step, next step, and percent complete. Use to check overall progress.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID to get progress for",
        },
      },
      required: ["task_id"],
    },
  },
];
