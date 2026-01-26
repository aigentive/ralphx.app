/**
 * ToolCallIndicator - Displays tool calls made by Claude during chat
 *
 * Features:
 * - Collapsible view (summary by default, expand for details)
 * - Shows tool name, icon, and brief summary
 * - Expands to show full arguments and result
 * - Special handling for artifact context tools (get_task_context, get_artifact)
 * - Styled with design system tokens for consistency
 */

import React, { useState } from "react";
import { Wrench, ChevronDown, ChevronRight, FileText, Package, Lightbulb } from "lucide-react";
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
function createSummary(toolCall: ToolCall): string {
  const { name, arguments: args, result } = toolCall;

  // Special formatting for common tools
  switch (name) {
    case "bash":
      return `Ran command: ${truncate(String((args as { command?: string })?.command || ""), 60)}`;
    case "read":
      return `Read file: ${(args as { file_path?: string })?.file_path || "unknown"}`;
    case "write":
      return `Wrote file: ${(args as { file_path?: string })?.file_path || "unknown"}`;
    case "edit":
      return `Edited file: ${(args as { file_path?: string })?.file_path || "unknown"}`;
    case "create_task_proposal":
      return `Created proposal: ${(args as { title?: string })?.title || "untitled"}`;
    case "update_task_proposal":
      return `Updated proposal: ${(args as { proposal_id?: string })?.proposal_id || "unknown"}`;
    case "delete_task_proposal":
      return `Deleted proposal: ${(args as { proposal_id?: string })?.proposal_id || "unknown"}`;
    case "update_task":
      return `Updated task: ${(args as { task_id?: string })?.task_id || "unknown"}`;
    case "add_task_note":
      return `Added note to task: ${(args as { task_id?: string })?.task_id || "unknown"}`;
    case "get_task_context": {
      // Extract task context from result
      const taskContext = result as TaskContext | undefined;
      if (taskContext?.task) {
        return `Fetched context for: ${(taskContext.task as { title?: string })?.title || "task"}`;
      }
      return `Fetched task context`;
    }
    case "get_artifact": {
      // Extract artifact from result
      const artifact = result as { title?: string } | undefined;
      if (artifact?.title) {
        return `Fetched artifact: ${artifact.title}`;
      }
      return `Fetched artifact`;
    }
    case "get_artifact_version":
      return `Fetched artifact version: ${(args as { version?: number })?.version || "unknown"}`;
    case "get_related_artifacts":
      return `Fetched related artifacts`;
    case "search_project_artifacts": {
      const query = (args as { query?: string })?.query;
      return query ? `Searched artifacts: "${query}"` : `Searched artifacts`;
    }
    default:
      return `Called ${name}`;
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

export function ToolCallIndicator({ toolCall, className = "" }: ToolCallIndicatorProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const summary = createSummary(toolCall);
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
        <Wrench
          size={14}
          className="flex-shrink-0"
          style={{ color: hasError ? "var(--text-primary)" : "var(--accent-primary)" }}
        />

        {/* Summary text */}
        <span
          className="text-xs flex-1"
          style={{
            color: hasError ? "var(--text-primary)" : "var(--text-secondary)",
            fontFamily: "var(--font-body)",
          }}
        >
          {summary}
        </span>

        {/* Error indicator */}
        {hasError && (
          <span
            className="text-xs font-medium"
            style={{ color: "var(--text-primary)" }}
          >
            Failed
          </span>
        )}
      </button>

      {/* Expanded details */}
      {isExpanded && (
        <div
          data-testid="tool-call-details"
          className="px-3 pb-3 space-y-3 border-t pt-3"
          style={{ borderColor: "var(--border-subtle)" }}
        >
          {/* Artifact preview for context tools */}
          {isArtifactContextTool(toolCall.name) && toolCall.result && !hasError ? (
            <div data-testid="artifact-preview">
              {renderArtifactPreview(toolCall)}
            </div>
          ) : null}

          {/* Collapsible raw data */}
          <details className="group">
            <summary
              className="text-xs font-medium cursor-pointer hover:opacity-80 transition-opacity list-none flex items-center gap-1"
              style={{ color: "var(--text-muted)" }}
            >
              <ChevronRight size={12} className="group-open:rotate-90 transition-transform" />
              Raw Data
            </summary>
            <div className="mt-2 space-y-2 pl-3">
              {/* Tool name */}
              <div>
                <div
                  className="text-xs font-medium mb-1"
                  style={{ color: "var(--text-muted)" }}
                >
                  Tool
                </div>
                <code
                  className="text-xs px-2 py-1 rounded block overflow-hidden text-ellipsis"
                  style={{
                    backgroundColor: "var(--bg-base)",
                    color: "var(--text-primary)",
                    fontFamily: "var(--font-mono)",
                  }}
                >
                  {toolCall.name}
                </code>
              </div>

              {/* Arguments */}
              <div>
                <div
                  className="text-xs font-medium mb-1"
                  style={{ color: "var(--text-muted)" }}
                >
                  Arguments
                </div>
                <pre
                  className="text-xs px-2 py-1 rounded overflow-x-auto max-w-full"
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

              {/* Result (if present and not null) */}
              {toolCall.result != null && !hasError && (
                <div>
                  <div
                    className="text-xs font-medium mb-1"
                    style={{ color: "var(--text-muted)" }}
                  >
                    Result
                  </div>
                  <pre
                    className="text-xs px-2 py-1 rounded overflow-x-auto max-w-full"
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
            </div>
          </details>

          {/* Error (if present) */}
          {hasError && (
            <div>
              <div
                className="text-xs font-medium mb-1"
                style={{ color: "var(--text-primary)" }}
              >
                Error
              </div>
              <pre
                className="text-xs px-2 py-1 rounded overflow-x-auto"
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
