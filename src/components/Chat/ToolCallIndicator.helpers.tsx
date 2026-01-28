/**
 * ToolCallIndicator helpers - Formatting and rendering logic for tool calls
 */

import React from "react";
import { Lightbulb, FileText, Package } from "lucide-react";
import type { TaskContext, ArtifactSummary } from "../../types/task-context";
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
      const taskContext = result as TaskContext | undefined;
      if (taskContext?.task) {
        return { title: (taskContext.task as { title?: string })?.title || "Fetched context" };
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
 * Format value for display
 * - Strings are displayed directly (preserving newlines)
 * - Objects/arrays are pretty-printed as JSON
 */
export function formatValue(value: unknown): { text: string; isPlainText: boolean } {
  if (typeof value === "string") {
    // String values are displayed directly - newlines will be preserved
    return { text: value, isPlainText: true };
  }
  try {
    return { text: JSON.stringify(value, null, 2), isPlainText: false };
  } catch {
    return { text: String(value), isPlainText: true };
  }
}

/**
 * Check if this tool call is for artifact context
 */
export function isArtifactContextTool(name: string): boolean {
  return ["get_task_context", "get_artifact", "get_artifact_version", "get_related_artifacts", "search_project_artifacts"].includes(name);
}

/**
 * Render artifact context preview for supported tools
 */
export function renderArtifactPreview(toolCall: ToolCall): React.ReactNode {
  const { name, result } = toolCall;

  if (!result) return null;

  switch (name) {
    case "get_task_context": {
      const taskContext = result as TaskContext;
      return (
        <div className="space-y-3">
          {/* Task info */}
          {taskContext.task && (
            <div>
              <div className="text-xs font-medium mb-1" style={{ color: "var(--text-muted)" }}>
                Task
              </div>
              <div className="text-xs px-2 py-1.5 rounded" style={{ backgroundColor: "var(--bg-base)", color: "var(--text-primary)" }}>
                {(taskContext.task as { title?: string })?.title || "Untitled"}
              </div>
            </div>
          )}

          {/* Source proposal */}
          {taskContext.sourceProposal && (
            <div>
              <div className="flex items-center gap-1 mb-1">
                <Lightbulb size={12} style={{ color: "var(--accent-primary)" }} />
                <span className="text-xs font-medium" style={{ color: "var(--text-muted)" }}>
                  Source Proposal
                </span>
              </div>
              <div className="text-xs px-2 py-1.5 rounded" style={{ backgroundColor: "var(--bg-base)" }}>
                <div style={{ color: "var(--text-primary)", fontWeight: 500 }}>
                  {taskContext.sourceProposal.title}
                </div>
                {taskContext.sourceProposal.description && (
                  <div className="mt-1" style={{ color: "var(--text-secondary)" }}>
                    {truncate(taskContext.sourceProposal.description, 100)}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Plan artifact */}
          {taskContext.planArtifact && (
            <div>
              <div className="flex items-center gap-1 mb-1">
                <FileText size={12} style={{ color: "var(--accent-primary)" }} />
                <span className="text-xs font-medium" style={{ color: "var(--text-muted)" }}>
                  Implementation Plan
                </span>
              </div>
              <div className="text-xs px-2 py-1.5 rounded" style={{ backgroundColor: "var(--bg-base)" }}>
                <div style={{ color: "var(--text-primary)", fontWeight: 500 }}>
                  {taskContext.planArtifact.title}
                </div>
                <div className="mt-1" style={{ color: "var(--text-secondary)", fontFamily: "var(--font-mono)", fontSize: "0.7rem" }}>
                  {truncate(taskContext.planArtifact.contentPreview, 150)}
                </div>
              </div>
            </div>
          )}

          {/* Related artifacts */}
          {taskContext.relatedArtifacts && taskContext.relatedArtifacts.length > 0 && (
            <div>
              <div className="flex items-center gap-1 mb-1">
                <Package size={12} style={{ color: "var(--text-muted)" }} />
                <span className="text-xs font-medium" style={{ color: "var(--text-muted)" }}>
                  Related Artifacts ({taskContext.relatedArtifacts.length})
                </span>
              </div>
              <div className="space-y-1">
                {taskContext.relatedArtifacts.slice(0, 3).map((artifact: ArtifactSummary, idx: number) => (
                  <div key={idx} className="text-xs px-2 py-1 rounded" style={{ backgroundColor: "var(--bg-base)", color: "var(--text-secondary)" }}>
                    {artifact.title}
                  </div>
                ))}
                {taskContext.relatedArtifacts.length > 3 && (
                  <div className="text-xs px-2 py-1" style={{ color: "var(--text-muted)" }}>
                    +{taskContext.relatedArtifacts.length - 3} more
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Context hints */}
          {taskContext.contextHints && taskContext.contextHints.length > 0 && (
            <div>
              <div className="text-xs font-medium mb-1" style={{ color: "var(--text-muted)" }}>
                Hints
              </div>
              <ul className="text-xs space-y-0.5 pl-4" style={{ color: "var(--text-secondary)" }}>
                {taskContext.contextHints.map((hint: string, idx: number) => (
                  <li key={idx} className="list-disc">{hint}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
      );
    }

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
