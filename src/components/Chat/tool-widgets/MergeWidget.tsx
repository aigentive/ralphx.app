/**
 * MergeWidget — Specialized renderers for merge-related MCP tools.
 *
 * Handles: complete_merge, report_conflict, report_incomplete, get_merge_target
 *
 * Design:
 * - complete_merge: green success card with commit SHA badge and branch info
 * - report_conflict: red card with conflict file list
 * - report_incomplete: amber/orange card with error details
 * - get_merge_target: compact card showing source → target branch arrows
 */

import React from "react";
import { GitMerge, GitBranch, AlertTriangle, ArrowRight, X } from "lucide-react";
import { WidgetCard, WidgetHeader, Badge, InlineIndicator, FilePath } from "./shared";
import { colors, getString, getStringArray, getBool } from "./shared.constants";
import type { ToolCallWidgetProps } from "./shared.constants";

/** Shorten a commit SHA to 7 chars */
function shortSha(sha: string): string {
  return sha.length > 7 ? sha.slice(0, 7) : sha;
}

/** Extract branch short name (last segment after /) */
function shortBranch(branch: string): string {
  const parts = branch.split("/");
  if (parts.length <= 1) return branch;
  // For "ralphx/slug/task-xxx" show "task-xxx"
  // For "main" show "main"
  return parts[parts.length - 1] || branch;
}

// ============================================================================
// complete_merge — Green success card
// ============================================================================

function CompleteMergeWidget({ toolCall, compact = false }: ToolCallWidgetProps) {
  const args = toolCall.arguments;
  const result = toolCall.result;

  const commitSha = getString(args, "commit_sha") ?? getString(result, "commit_sha");
  const success = getBool(result, "success");
  const message = getString(result, "message");
  const newStatus = getString(result, "new_status");

  // If tool errored, show inline error
  if (toolCall.error) {
    return (
      <InlineIndicator
        icon={<X size={12} style={{ color: colors.error }} />}
        text={`Merge failed: ${toolCall.error}`}
      />
    );
  }

  // Result not available yet
  if (result == null) {
    return (
      <InlineIndicator
        icon={<GitMerge size={12} style={{ color: colors.textMuted }} />}
        text="Completing merge..."
      />
    );
  }

  return (
    <div
      style={{
        background: colors.successDim,
        borderRadius: 10,
        border: `1px solid hsla(145 60% 45% / 0.20)`,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: compact ? "6px 10px" : "8px 12px",
        }}
      >
        {/* Merge icon */}
        <GitMerge size={14} style={{ color: colors.success, flexShrink: 0 }} />

        {/* Title */}
        <span
          style={{
            fontSize: compact ? 11 : 11.5,
            fontWeight: 500,
            color: colors.success,
            flex: 1,
          }}
        >
          {success === false ? "Merge failed" : "Merge completed"}
        </span>

        {/* Commit SHA badge */}
        {commitSha && (
          <span
            style={{
              fontSize: 9.5,
              fontFamily: "var(--font-mono)",
              padding: "1px 6px",
              borderRadius: 6,
              fontWeight: 500,
              background: "hsla(145 60% 45% / 0.15)",
              color: colors.success,
              flexShrink: 0,
            }}
          >
            {shortSha(commitSha)}
          </span>
        )}

        {/* Status badge */}
        {newStatus && (
          <Badge variant="success" compact>{newStatus}</Badge>
        )}
      </div>

      {/* Message detail */}
      {message && (
        <div
          style={{
            fontSize: 10,
            color: "hsl(145 30% 55%)",
            padding: compact ? "0 10px 6px" : "0 12px 8px",
            paddingTop: 0,
          }}
        >
          {message}
        </div>
      )}
    </div>
  );
}

// ============================================================================
// report_conflict — Red card with conflict file list
// ============================================================================

function ReportConflictWidget({ toolCall, compact = false }: ToolCallWidgetProps) {
  const args = toolCall.arguments;

  const conflictFiles = getStringArray(args, "conflict_files") ?? [];
  const reason = getString(args, "reason");

  if (toolCall.error) {
    return (
      <InlineIndicator
        icon={<X size={12} style={{ color: colors.error }} />}
        text={`Report conflict failed: ${toolCall.error}`}
      />
    );
  }

  const fileCount = conflictFiles.length;
  const fewFiles = fileCount <= 3;

  return (
    <WidgetCard
      compact={compact}
      alwaysExpanded={fewFiles}
      header={
        <WidgetHeader
          icon={<AlertTriangle size={14} style={{ color: colors.error }} />}
          title={reason ? `Conflict: ${reason}` : "Merge conflict"}
          badge={
            <Badge variant="error" compact>
              {fileCount} {fileCount === 1 ? "file" : "files"}
            </Badge>
          }
          compact={compact}
        />
      }
    >
      {/* Conflict file list */}
      <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        {conflictFiles.map((file, i) => (
          <div
            key={i}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              padding: "2px 0",
            }}
          >
            <X size={10} style={{ color: colors.error, flexShrink: 0 }} />
            <FilePath path={file} />
          </div>
        ))}
        {fileCount === 0 && (
          <span style={{ fontSize: 10.5, color: colors.textMuted }}>
            No conflict files listed
          </span>
        )}
      </div>
    </WidgetCard>
  );
}

// ============================================================================
// report_incomplete — Amber/orange card with error details
// ============================================================================

function ReportIncompleteWidget({ toolCall, compact = false }: ToolCallWidgetProps) {
  const args = toolCall.arguments;

  const reason = getString(args, "reason");
  const diagnosticInfo = getString(args, "diagnostic_info");

  if (toolCall.error) {
    return (
      <InlineIndicator
        icon={<X size={12} style={{ color: colors.error }} />}
        text={`Report incomplete failed: ${toolCall.error}`}
      />
    );
  }

  return (
    <WidgetCard
      compact={compact}
      alwaysExpanded={!diagnosticInfo}
      header={
        <WidgetHeader
          icon={<AlertTriangle size={14} style={{ color: colors.accent }} />}
          title={reason ?? "Merge incomplete"}
          badge={<Badge variant="accent" compact>incomplete</Badge>}
          compact={compact}
        />
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {/* Reason */}
        {reason && (
          <div style={{ fontSize: 11, color: colors.textSecondary, lineHeight: 1.4 }}>
            {reason}
          </div>
        )}

        {/* Diagnostic info (code block) */}
        {diagnosticInfo && (
          <pre
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              color: colors.textMuted,
              background: colors.bgTerminal,
              padding: "6px 8px",
              borderRadius: 6,
              overflow: "auto",
              maxHeight: compact ? 80 : 120,
              whiteSpace: "pre-wrap",
              wordBreak: "break-word",
              margin: 0,
            }}
          >
            {diagnosticInfo}
          </pre>
        )}
      </div>
    </WidgetCard>
  );
}

// ============================================================================
// get_merge_target — Compact source → target branch card
// ============================================================================

function GetMergeTargetWidget({ toolCall, compact = false }: ToolCallWidgetProps) {
  const result = toolCall.result;

  const sourceBranch = getString(result, "source_branch");
  const targetBranch = getString(result, "target_branch");

  if (toolCall.error) {
    return (
      <InlineIndicator
        icon={<X size={12} style={{ color: colors.error }} />}
        text={`Get merge target failed: ${toolCall.error}`}
      />
    );
  }

  // Not loaded yet
  if (!sourceBranch || !targetBranch) {
    return (
      <InlineIndicator
        icon={<GitBranch size={12} style={{ color: colors.textMuted }} />}
        text="Resolving merge target..."
      />
    );
  }

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        padding: compact ? "4px 10px" : "6px 10px",
        margin: "2px 0",
      }}
    >
      {/* Git branch icon */}
      <GitBranch size={12} style={{ color: colors.blue, flexShrink: 0 }} />

      {/* Source branch */}
      <span
        style={{
          fontSize: compact ? 10 : 10.5,
          fontFamily: "var(--font-mono)",
          color: colors.textSecondary,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
        title={sourceBranch}
      >
        {shortBranch(sourceBranch)}
      </span>

      {/* Arrow */}
      <ArrowRight size={10} style={{ color: colors.textMuted, flexShrink: 0 }} />

      {/* Target branch */}
      <span
        style={{
          fontSize: compact ? 10 : 10.5,
          fontFamily: "var(--font-mono)",
          color: colors.textPrimary,
          fontWeight: 500,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
        title={targetBranch}
      >
        {shortBranch(targetBranch)}
      </span>

      {/* Merge badge */}
      <Badge variant="blue" compact>merge target</Badge>
    </div>
  );
}

// ============================================================================
// Main MergeWidget — Dispatcher
// ============================================================================

export const MergeWidget = React.memo(function MergeWidget(props: ToolCallWidgetProps) {
  const toolName = props.toolCall.name.toLowerCase();

  switch (toolName) {
    case "mcp__ralphx__complete_merge":
    case "complete_merge":
      return <CompleteMergeWidget {...props} />;

    case "mcp__ralphx__report_conflict":
    case "report_conflict":
      return <ReportConflictWidget {...props} />;

    case "mcp__ralphx__report_incomplete":
    case "report_incomplete":
      return <ReportIncompleteWidget {...props} />;

    case "mcp__ralphx__get_merge_target":
    case "get_merge_target":
      return <GetMergeTargetWidget {...props} />;

    default:
      return (
        <InlineIndicator
          icon={<GitMerge size={12} style={{ color: colors.textMuted }} />}
          text={props.toolCall.name}
        />
      );
  }
});
