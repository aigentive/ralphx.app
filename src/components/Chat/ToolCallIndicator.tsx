/**
 * ToolCallIndicator - Displays tool calls made by Claude during chat (final render)
 *
 * Features:
 * - Collapsible view (summary by default, expand for raw data)
 * - Shows tool name and smart summary extracted from arguments
 * - Expands to show formatted JSON (no nested collapse)
 * - Compact, readable design
 */

import React, { useState, useMemo } from "react";
import { Wrench, ChevronDown, ChevronRight, FileText, Package, Lightbulb, Terminal, FileEdit, Search, FolderSearch } from "lucide-react";
import type { TaskContext, ArtifactSummary } from "../../types/task-context";

// ============================================================================
// Types
// ============================================================================

/**
 * Tool call structure from Claude CLI stream-json output
 */
export interface ToolCall {
  /** Unique identifier for this tool call */
  id: string;
  /** Name of the tool that was called */
  name: string;
  /** Arguments passed to the tool (can be any JSON value) */
  arguments: unknown;
  /** Result returned from the tool (can be any JSON value) */
  result?: unknown;
  /** Error message if tool call failed */
  error?: string;
}

interface ToolCallIndicatorProps {
  /** The tool call to display */
  toolCall: ToolCall;
  /** Optional additional className for container */
  className?: string;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Create a brief summary of the tool call for collapsed view
 */
function createSummary(toolCall: ToolCall): { title: string; subtitle?: string | undefined } {
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
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + "...";
}

/**
 * Format JSON for display
 */
function formatJSON(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

/**
 * Check if this tool call is for artifact context
 */
function isArtifactContextTool(name: string): boolean {
  return ["get_task_context", "get_artifact", "get_artifact_version", "get_related_artifacts", "search_project_artifacts"].includes(name);
}

/**
 * Render artifact context preview for supported tools
 */
function renderArtifactPreview(toolCall: ToolCall): React.ReactNode {
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

// ============================================================================
// Component
// ============================================================================

/**
 * Render tool icon based on tool name
 */
function ToolIcon({ name, hasError }: { name: string; hasError: boolean }) {
  const style = { color: hasError ? "var(--text-primary)" : "var(--accent-primary)" };
  const className = "flex-shrink-0";

  switch (name) {
    case "bash":
      return <Terminal size={14} className={className} style={style} />;
    case "read":
    case "write":
      return <FileText size={14} className={className} style={style} />;
    case "edit":
      return <FileEdit size={14} className={className} style={style} />;
    case "glob":
      return <FolderSearch size={14} className={className} style={style} />;
    case "grep":
      return <Search size={14} className={className} style={style} />;
    default:
      return <Wrench size={14} className={className} style={style} />;
  }
}

export function ToolCallIndicator({ toolCall, className = "" }: ToolCallIndicatorProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const summary = useMemo(() => createSummary(toolCall), [toolCall]);
  const hasError = Boolean(toolCall.error);

  return (
    <div
      data-testid="tool-call-indicator"
      className={`rounded-md border overflow-hidden max-w-full ${className}`}
      style={{
        backgroundColor: hasError ? "var(--status-error)" : "var(--bg-elevated)",
        borderColor: hasError
          ? "rgba(239, 68, 68, 0.3)"
          : "var(--border-subtle)",
        opacity: hasError ? 0.9 : 1,
      }}
    >
      {/* Header - Always visible */}
      <button
        data-testid="tool-call-toggle"
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-center gap-2 px-3 py-2 text-left hover:opacity-80 transition-opacity"
        aria-expanded={isExpanded}
        aria-label={`Tool call: ${toolCall.name}. Click to ${isExpanded ? "collapse" : "expand"} details.`}
      >
        {/* Expand/collapse icon */}
        {isExpanded ? (
          <ChevronDown
            size={14}
            className="flex-shrink-0"
            style={{ color: "var(--text-muted)" }}
          />
        ) : (
          <ChevronRight
            size={14}
            className="flex-shrink-0"
            style={{ color: "var(--text-muted)" }}
          />
        )}

        {/* Tool icon */}
        <ToolIcon name={toolCall.name} hasError={hasError} />

        {/* Tool name badge */}
        <span
          className="text-[10px] px-1.5 py-0.5 rounded flex-shrink-0"
          style={{
            backgroundColor: hasError ? "rgba(0,0,0,0.2)" : "var(--bg-base)",
            color: hasError ? "var(--text-primary)" : "var(--text-muted)",
            fontFamily: "var(--font-mono)",
          }}
        >
          {toolCall.name}
        </span>

        {/* Summary text */}
        <div className="flex-1 min-w-0 flex flex-col">
          <span
            className="text-xs truncate font-mono"
            style={{
              color: hasError ? "var(--text-primary)" : "var(--text-secondary)",
            }}
          >
            {summary.title}
          </span>
          {summary.subtitle && (
            <span
              className="text-[10px] truncate"
              style={{ color: "var(--text-muted)" }}
            >
              {summary.subtitle}
            </span>
          )}
        </div>

        {/* Error indicator */}
        {hasError && (
          <span
            className="text-[10px] font-medium px-1.5 py-0.5 rounded"
            style={{
              backgroundColor: "rgba(0,0,0,0.2)",
              color: "var(--text-primary)",
            }}
          >
            Failed
          </span>
        )}
      </button>

      {/* Expanded details - NO nested collapse, show raw data directly */}
      {isExpanded && (
        <div
          data-testid="tool-call-details"
          className="px-3 pb-3 space-y-2 border-t pt-3"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          {/* Artifact preview for context tools */}
          {isArtifactContextTool(toolCall.name) && toolCall.result && !hasError ? (
            <div data-testid="artifact-preview" className="mb-3">
              {renderArtifactPreview(toolCall)}
            </div>
          ) : null}

          {/* Arguments - shown directly */}
          <div>
            <div
              className="text-[10px] font-medium mb-1 uppercase tracking-wide"
              style={{ color: "var(--text-muted)" }}
            >
              Arguments
            </div>
            <pre
              className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-w-full max-h-48"
              style={{
                backgroundColor: "var(--bg-base)",
                color: "var(--text-primary)",
                fontFamily: "var(--font-mono)",
                wordBreak: "break-word",
                whiteSpace: "pre-wrap",
              }}
            >
              {formatJSON(toolCall.arguments)}
            </pre>
          </div>

          {/* Result - shown directly (if present and not null) */}
          {toolCall.result != null && !hasError && (
            <div>
              <div
                className="text-[10px] font-medium mb-1 uppercase tracking-wide"
                style={{ color: "var(--text-muted)" }}
              >
                Result
              </div>
              <pre
                className="text-[11px] px-2 py-1.5 rounded overflow-x-auto max-w-full max-h-48"
                style={{
                  backgroundColor: "var(--bg-base)",
                  color: "var(--text-primary)",
                  fontFamily: "var(--font-mono)",
                  wordBreak: "break-word",
                  whiteSpace: "pre-wrap",
                }}
              >
                {formatJSON(toolCall.result)}
              </pre>
            </div>
          )}

          {/* Error (if present) */}
          {hasError && (
            <div>
              <div
                className="text-[10px] font-medium mb-1 uppercase tracking-wide"
                style={{ color: "var(--text-primary)" }}
              >
                Error
              </div>
              <pre
                className="text-[11px] px-2 py-1.5 rounded overflow-x-auto"
                style={{
                  backgroundColor: "rgba(0, 0, 0, 0.2)",
                  color: "var(--text-primary)",
                  fontFamily: "var(--font-mono)",
                }}
              >
                {toolCall.error}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
