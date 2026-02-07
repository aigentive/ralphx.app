/**
 * MCP tool definitions for review issue management
 * Used by worker agent (during re-execution) and reviewer agent (during re-reviews)
 */

import { Tool } from "@modelcontextprotocol/sdk/types.js";

/**
 * Issue tools for worker and reviewer agents
 * All tools are proxies that forward to Tauri backend via HTTP
 */
export const ISSUE_TOOLS: Tool[] = [
  {
    name: "get_task_issues",
    description:
      "Fetch review issues for a task. Returns structured issues from prior reviews with severity, status, file path, and line number. " +
      "Use status_filter='open' to see only unresolved issues. Call this at the start of re-execution to understand what needs to be fixed.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The ID of the task to get issues for",
        },
        status_filter: {
          type: "string",
          enum: ["open", "all"],
          description:
            "Filter issues by status: 'open' for unresolved only, 'all' for everything (default: 'all')",
        },
      },
      required: ["task_id"],
    },
  },
  {
    name: "get_issue_progress",
    description:
      "Get a summary of issue resolution progress for a task. Returns counts of open, in_progress, and addressed issues. " +
      "Use during re-review to quickly check how many issues the worker addressed.",
    inputSchema: {
      type: "object",
      properties: {
        task_id: {
          type: "string",
          description: "The task ID to get issue progress for",
        },
      },
      required: ["task_id"],
    },
  },
  {
    name: "mark_issue_in_progress",
    description:
      "Mark a review issue as in-progress. Call this BEFORE starting to fix an issue to track progress. " +
      "Only open issues can be marked in-progress.",
    inputSchema: {
      type: "object",
      properties: {
        issue_id: {
          type: "string",
          description: "The ID of the issue to mark as in-progress",
        },
      },
      required: ["issue_id"],
    },
  },
  {
    name: "mark_issue_addressed",
    description:
      "Mark a review issue as addressed. Call this AFTER fixing an issue to indicate it has been resolved. " +
      "Provide resolution notes explaining what was done. Only open or in-progress issues can be marked addressed.",
    inputSchema: {
      type: "object",
      properties: {
        issue_id: {
          type: "string",
          description: "The ID of the issue to mark as addressed",
        },
        resolution_notes: {
          type: "string",
          description:
            "Description of how the issue was resolved (e.g., 'Fixed null check in validateInput()')",
        },
        attempt_number: {
          type: "number",
          description: "The current re-execution attempt number (starts at 1)",
        },
      },
      required: ["issue_id", "resolution_notes", "attempt_number"],
    },
  },
];
