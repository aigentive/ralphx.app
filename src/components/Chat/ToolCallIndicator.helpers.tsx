/**
 * ToolCallIndicator helpers - Formatting and rendering logic for tool calls
 */

import type { ToolCall } from "./ToolCallIndicator";

/**
 * Create a brief summary of the tool call for collapsed view
 */
export function createSummary(toolCall: ToolCall): { title: string; subtitle?: string | undefined } {
  const { name, arguments: args } = toolCall;
  // Normalize tool name to lowercase for matching
  const normalizedName = name.toLowerCase();

  // Specialized formatting for tools that still use the generic fallback renderer.
  // Tools with dedicated widgets (bash, read, grep, glob, step tools, context,
  // artifacts, reviews, proposals, merges, ideation) are handled by their widgets
  // and never reach this function.
  switch (normalizedName) {
    case "write":
      return { title: (args as { file_path?: string })?.file_path || "Wrote file" };
    case "edit":
      return { title: (args as { file_path?: string })?.file_path || "Edited file" };
    case "update_task":
      return { title: "Updated task" };
    case "add_task_note":
      return { title: "Added note" };
    default: {
      // For unknown tools, just show the tool name in readable form
      return { title: name.replace(/_/g, " ") };
    }
  }
}

/**
 * Strip ANSI escape codes from text
 * Handles color codes, cursor movement, and other terminal sequences
 */
export function stripAnsiCodes(text: string): string {
  // Match ANSI escape sequences:
  // - \x1b[ or \033[ followed by parameters and a letter
  // - Also handles OSC sequences (\x1b]) and other escape sequences
  // eslint-disable-next-line no-control-regex
  return text.replace(/\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07|\x1b[PX^_][^\x1b]*\x1b\\|\x1b./g, '');
}

/**
 * Format value for display
 * - Strings are displayed directly (preserving newlines)
 * - Objects/arrays are pretty-printed as JSON
 * - ANSI escape codes are stripped from all text output
 */
export function formatValue(value: unknown): { text: string; isPlainText: boolean } {
  if (typeof value === "string") {
    // String values are displayed directly - newlines will be preserved
    // Strip ANSI escape codes for clean display
    return { text: stripAnsiCodes(value), isPlainText: true };
  }
  try {
    const json = JSON.stringify(value, null, 2);
    return { text: stripAnsiCodes(json), isPlainText: false };
  } catch {
    return { text: stripAnsiCodes(String(value)), isPlainText: true };
  }
}

