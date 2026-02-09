/**
 * ToolCallIndicator helpers - Formatting and rendering logic for tool calls
 */

import React from "react";
import type { ArtifactSummary } from "../../types/task-context";
import type { ToolCall } from "./ToolCallIndicator";

/**
 * Create a brief summary of the tool call for collapsed view
 */
export function createSummary(toolCall: ToolCall): { title: string; subtitle?: string | undefined } {
  const { name, arguments: args, result } = toolCall;
  // Normalize tool name to lowercase for matching
  const normalizedName = name.toLowerCase();

  // Special formatting for common tools
  switch (normalizedName) {
    case "bash": {
      const typedArgs = args as { command?: string; description?: string } | undefined;
      const desc = typedArgs?.description;
      const cmd = typedArgs?.command;
      if (desc) {
        return { title: desc, subtitle: cmd ? truncate(cmd, 60) : undefined };
      }
      return { title: cmd ? truncate(cmd, 80) : "Ran command" };
    }
    case "read":
      return { title: (args as { file_path?: string })?.file_path || "Read file" };
    case "write":
      return { title: (args as { file_path?: string })?.file_path || "Wrote file" };
    case "edit":
      return { title: (args as { file_path?: string })?.file_path || "Edited file" };
    case "glob": {
      const typedArgs = args as { pattern?: string; path?: string } | undefined;
      return {
        title: typedArgs?.pattern || "Search files",
        subtitle: typedArgs?.path,
      };
    }
    case "grep": {
      const typedArgs = args as { pattern?: string; path?: string } | undefined;
      const pattern = typedArgs?.pattern;
      const path = typedArgs?.path;
      if (pattern && path) {
        return {
          title: `"${truncate(pattern, 30)}"`,
          subtitle: `in ${path}`,
        };
      } else if (pattern) {
        return { title: `"${truncate(pattern, 40)}"` };
      } else if (path) {
        return { title: `Search in ${path}` };
      }
      return { title: "Search content" };
    }
    case "create_task_proposal":
      return { title: (args as { title?: string })?.title || "Created proposal" };
    case "update_task_proposal":
      return { title: (args as { title?: string })?.title || "Updated proposal" };
    case "delete_task_proposal":
      return { title: "Deleted proposal" };
    case "update_task":
      return { title: "Updated task" };
    case "add_task_note":
      return { title: "Added note" };
    case "get_task_context": {
      const ctx = result as { task?: { title?: string } } | undefined;
      if (ctx?.task?.title) {
        return { title: ctx.task.title };
      }
      return { title: "Fetched task context" };
    }
    case "get_artifact": {
      const artifact = result as { title?: string } | undefined;
      return { title: artifact?.title || "Fetched artifact" };
    }
    case "get_artifact_version":
      return { title: `Version ${(args as { version?: number })?.version || "?"}` };
    case "get_related_artifacts":
      return { title: "Fetched related artifacts" };
    case "search_project_artifacts": {
      const query = (args as { query?: string })?.query;
      return { title: query ? `"${truncate(query, 40)}"` : "Searched artifacts" };
    }
    default: {
      // For unknown tools, just show the tool name in readable form
      return { title: name.replace(/_/g, " ") };
    }
  }
}

/**
 * Truncate text with ellipsis
 */
export function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + "...";
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

/**
 * Check if this tool call is for artifact context
 */
export function isArtifactContextTool(name: string): boolean {
  return ["get_artifact", "get_artifact_version", "get_related_artifacts", "search_project_artifacts"].includes(name);
}

/**
 * Render artifact context preview for supported tools
 */
export function renderArtifactPreview(toolCall: ToolCall): React.ReactNode {
  const { name, result } = toolCall;

  if (!result) return null;

  switch (name) {
    case "get_artifact": {
      const artifact = result as { id?: string; title?: string; artifactType?: string; content?: string };
      return (
        <div className="space-y-2">
          {artifact.title && (
            <div>
              <div className="text-xs font-medium mb-1" style={{ color: "var(--text-muted)" }}>
                Title
              </div>
              <div className="text-xs px-2 py-1.5 rounded" style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)", fontWeight: 500 }}>
                {artifact.title}
              </div>
            </div>
          )}
          {artifact.artifactType && (
            <div>
              <div className="text-xs font-medium mb-1" style={{ color: "var(--text-muted)" }}>
                Type
              </div>
              <div className="text-xs px-2 py-1 rounded inline-block" style={{ backgroundColor: "var(--bg-base)", color: "var(--text-secondary)" }}>
                {artifact.artifactType}
              </div>
            </div>
          )}
          {artifact.content && (
            <div>
              <div className="text-xs font-medium mb-1" style={{ color: "var(--text-muted)" }}>
                Content Preview
              </div>
              <pre className="text-xs px-2 py-1.5 rounded overflow-x-auto" style={{ backgroundColor: "var(--bg-base)", color: "var(--text-secondary)", fontFamily: "var(--font-mono)" }}>
                {truncate(artifact.content, 300)}
              </pre>
            </div>
          )}
        </div>
      );
    }

    case "get_related_artifacts": {
      const artifacts = result as ArtifactSummary[] | undefined;
      if (!artifacts || artifacts.length === 0) {
        return <div className="text-xs" style={{ color: "var(--text-muted)" }}>No related artifacts found</div>;
      }
      return (
        <div>
          <div className="text-xs font-medium mb-2" style={{ color: "var(--text-muted)" }}>
            Found {artifacts.length} artifact{artifacts.length !== 1 ? 's' : ''}
          </div>
          <div className="space-y-1">
            {artifacts.slice(0, 5).map((artifact: ArtifactSummary, idx: number) => (
              <div key={idx} className="text-xs px-2 py-1.5 rounded" style={{ backgroundColor: "var(--bg-base)" }}>
                <div style={{ color: "var(--text-primary)", fontWeight: 500 }}>{artifact.title}</div>
                <div style={{ color: "var(--text-muted)", fontSize: "0.7rem" }}>{artifact.artifactType}</div>
              </div>
            ))}
            {artifacts.length > 5 && (
              <div className="text-xs px-2 py-1" style={{ color: "var(--text-muted)" }}>
                +{artifacts.length - 5} more
              </div>
            )}
          </div>
        </div>
      );
    }

    case "search_project_artifacts": {
      const artifacts = result as ArtifactSummary[] | undefined;
      if (!artifacts || artifacts.length === 0) {
        return <div className="text-xs" style={{ color: "var(--text-muted)" }}>No artifacts found</div>;
      }
      return (
        <div>
          <div className="text-xs font-medium mb-2" style={{ color: "var(--text-muted)" }}>
            Found {artifacts.length} result{artifacts.length !== 1 ? 's' : ''}
          </div>
          <div className="space-y-1">
            {artifacts.slice(0, 5).map((artifact: ArtifactSummary, idx: number) => (
              <div key={idx} className="text-xs px-2 py-1.5 rounded" style={{ backgroundColor: "var(--bg-base)" }}>
                <div style={{ color: "var(--text-primary)", fontWeight: 500 }}>{artifact.title}</div>
                <div style={{ color: "var(--text-muted)", fontSize: "0.7rem" }}>{artifact.artifactType}</div>
                {artifact.contentPreview && (
                  <div className="mt-1" style={{ color: "var(--text-secondary)", fontSize: "0.65rem" }}>
                    {truncate(artifact.contentPreview, 80)}
                  </div>
                )}
              </div>
            ))}
            {artifacts.length > 5 && (
              <div className="text-xs px-2 py-1" style={{ color: "var(--text-muted)" }}>
                +{artifacts.length - 5} more
              </div>
            )}
          </div>
        </div>
      );
    }

    default:
      return null;
  }
}
