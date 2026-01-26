/**
 * StreamingToolIndicator - Live tool call display during agent execution
 *
 * Shows a single, always-expanded card that displays a chain-of-thought
 * built from incoming tool calls. Each tool call is shown as a summary line
 * with the tool name and key argument values extracted dynamically.
 *
 * This is used ONLY during streaming - final messages use ToolCallIndicator.
 */

import { useMemo } from "react";
import { Wrench, Loader2 } from "lucide-react";
import type { ToolCall } from "./ToolCallIndicator";

// ============================================================================
// Types
// ============================================================================

interface StreamingToolIndicatorProps {
  /** Tool calls accumulated during streaming */
  toolCalls: ToolCall[];
  /** Whether the agent is still running */
  isActive?: boolean;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Extract a readable summary from tool arguments
 * Only uses first-level keys that are not nested objects
 */
function extractArgumentSummary(args: unknown): string[] {
  if (!args || typeof args !== "object" || Array.isArray(args)) {
    return [];
  }

  const summaries: string[] = [];
  const entries = Object.entries(args as Record<string, unknown>);

  for (const [key, value] of entries) {
    // Skip nested objects and arrays
    if (value && typeof value === "object") {
      continue;
    }

    // Skip null/undefined
    if (value == null) {
      continue;
    }

    // Format the value
    const strValue = String(value);
    const truncated = strValue.length > 60 ? strValue.slice(0, 57) + "..." : strValue;

    // Format the key (snake_case to readable)
    const readableKey = key.replace(/_/g, " ");

    summaries.push(`${readableKey}: ${truncated}`);
  }

  return summaries.slice(0, 3); // Max 3 key-value pairs
}

/**
 * Create a one-line summary for a tool call
 */
function createToolSummary(toolCall: ToolCall): { primary: string; details: string[] } {
  const { name, arguments: args } = toolCall;
  // Normalize tool name to lowercase for matching
  const normalizedName = name.toLowerCase();

  // Known tool formats with primary info extraction
  switch (normalizedName) {
    case "bash": {
      const typedArgs = args as { command?: string; description?: string } | undefined;
      const desc = typedArgs?.description;
      const cmd = typedArgs?.command;
      return {
        primary: desc || (cmd ? `$ ${cmd.slice(0, 50)}${cmd.length > 50 ? "..." : ""}` : "Running command"),
        details: desc && cmd ? [`$ ${cmd.slice(0, 60)}${cmd.length > 60 ? "..." : ""}`] : [],
      };
    }
    case "read": {
      const typedArgs = args as { file_path?: string } | undefined;
      return {
        primary: typedArgs?.file_path || "Reading file",
        details: [],
      };
    }
    case "write": {
      const typedArgs = args as { file_path?: string } | undefined;
      return {
        primary: typedArgs?.file_path || "Writing file",
        details: [],
      };
    }
    case "edit": {
      const typedArgs = args as { file_path?: string } | undefined;
      return {
        primary: typedArgs?.file_path || "Editing file",
        details: [],
      };
    }
    case "glob": {
      const typedArgs = args as { pattern?: string; path?: string } | undefined;
      return {
        primary: typedArgs?.pattern || "Searching files",
        details: typedArgs?.path ? [`in ${typedArgs.path}`] : [],
      };
    }
    case "grep": {
      const typedArgs = args as { pattern?: string; path?: string } | undefined;
      return {
        primary: typedArgs?.pattern ? `"${typedArgs.pattern}"` : "Searching content",
        details: typedArgs?.path ? [`in ${typedArgs.path}`] : [],
      };
    }
    case "create_task_proposal": {
      const typedArgs = args as { title?: string } | undefined;
      return {
        primary: typedArgs?.title || "Creating proposal",
        details: [],
      };
    }
    case "update_task_proposal": {
      const typedArgs = args as { proposal_id?: string; title?: string } | undefined;
      return {
        primary: typedArgs?.title || `Updating ${typedArgs?.proposal_id?.slice(0, 8) || "proposal"}`,
        details: [],
      };
    }
    case "get_task_context": {
      return {
        primary: "Fetching task context",
        details: [],
      };
    }
    default: {
      // Dynamic extraction for unknown tools
      const details = extractArgumentSummary(args);
      return {
        primary: name.replace(/_/g, " "),
        details,
      };
    }
  }
}

/**
 * Get a verb for the tool action
 */
function getToolVerb(name: string): string {
  const normalizedName = name.toLowerCase();
  switch (normalizedName) {
    case "bash":
      return "Running";
    case "read":
      return "Reading";
    case "write":
      return "Writing";
    case "edit":
      return "Editing";
    case "glob":
      return "Finding";
    case "grep":
      return "Searching";
    case "create_task_proposal":
      return "Creating";
    case "update_task_proposal":
      return "Updating";
    case "delete_task_proposal":
      return "Deleting";
    case "get_task_context":
    case "get_artifact":
      return "Fetching";
    default:
      return "Calling";
  }
}

// ============================================================================
// Component
// ============================================================================

export function StreamingToolIndicator({
  toolCalls,
  isActive = true,
}: StreamingToolIndicatorProps) {
  // Process tool calls into summary lines
  const summaryLines = useMemo(() => {
    return toolCalls.map((tc) => {
      const { primary, details } = createToolSummary(tc);
      const verb = getToolVerb(tc.name);
      return {
        id: tc.id,
        name: tc.name,
        verb,
        primary,
        details,
        hasError: Boolean(tc.error),
      };
    });
  }, [toolCalls]);

  if (summaryLines.length === 0) {
    return null;
  }

  return (
    <div
      data-testid="streaming-tool-indicator"
      className="rounded-lg overflow-hidden mb-2"
      style={{
        backgroundColor: "var(--bg-elevated)",
        border: "1px solid var(--border-subtle)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-2 border-b"
        style={{ borderColor: "var(--border-subtle)" }}
      >
        {isActive ? (
          <Loader2
            size={14}
            className="animate-spin flex-shrink-0"
            style={{ color: "var(--accent-primary)" }}
          />
        ) : (
          <Wrench
            size={14}
            className="flex-shrink-0"
            style={{ color: "var(--accent-primary)" }}
          />
        )}
        <span
          className="text-xs font-medium"
          style={{ color: "var(--text-secondary)" }}
        >
          {isActive ? "Working..." : `${summaryLines.length} tool call${summaryLines.length !== 1 ? "s" : ""}`}
        </span>
      </div>

      {/* Chain of thought - tool call summaries */}
      <div className="px-3 py-2 space-y-1.5">
        {summaryLines.map((line, index) => (
          <div
            key={line.id || index}
            className="flex items-start gap-2 text-xs"
            style={{
              color: line.hasError ? "var(--status-error)" : "var(--text-secondary)",
            }}
          >
            {/* Step indicator */}
            <span
              className="flex-shrink-0 w-4 text-right tabular-nums"
              style={{ color: "var(--text-muted)" }}
            >
              {index + 1}.
            </span>

            {/* Summary content */}
            <div className="flex-1 min-w-0">
              <span className="font-medium" style={{ color: "var(--text-primary)" }}>
                {line.verb}
              </span>
              {" "}
              <span
                className="font-mono break-all"
                style={{
                  color: line.hasError ? "var(--status-error)" : "var(--text-secondary)",
                }}
              >
                {line.primary}
              </span>

              {/* Additional details */}
              {line.details.length > 0 && (
                <div
                  className="mt-0.5 text-[11px] font-mono"
                  style={{ color: "var(--text-muted)" }}
                >
                  {line.details.map((detail, i) => (
                    <div key={i} className="truncate">
                      {detail}
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ))}

        {/* Active indicator at the end */}
        {isActive && (
          <div className="flex items-center gap-2 text-xs pt-1">
            <span className="w-4" />
            <div className="flex items-center gap-1">
              <div
                className="w-1.5 h-1.5 rounded-full animate-pulse"
                style={{ backgroundColor: "var(--accent-primary)" }}
              />
              <div
                className="w-1.5 h-1.5 rounded-full animate-pulse"
                style={{ backgroundColor: "var(--accent-primary)", animationDelay: "0.15s" }}
              />
              <div
                className="w-1.5 h-1.5 rounded-full animate-pulse"
                style={{ backgroundColor: "var(--accent-primary)", animationDelay: "0.3s" }}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
