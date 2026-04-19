import React, { useMemo } from "react";
import { FileEdit } from "lucide-react";

import { Badge, FilePath, WidgetCard, WidgetHeader, WidgetRow } from "./shared";
import type { ToolCallWidgetProps } from "./shared.constants";

interface FileChangeEntry {
  path?: string;
  kind?: string;
}

function parseChanges(toolCall: ToolCallWidgetProps["toolCall"]): FileChangeEntry[] {
  const args = toolCall.arguments;
  if (args == null || typeof args !== "object") {
    return [];
  }

  const changes = (args as { changes?: unknown }).changes;
  if (!Array.isArray(changes)) {
    return [];
  }

  return changes.filter(
    (change): change is FileChangeEntry =>
      change != null
      && typeof change === "object"
      && typeof (change as { path?: unknown }).path === "string",
  );
}

function kindLabel(kind?: string): string {
  switch (kind) {
    case "add":
    case "create":
      return "created";
    case "delete":
      return "deleted";
    case "rename":
      return "renamed";
    case "update":
      return "updated";
    default:
      return "changed";
  }
}

function headerTitle(changes: FileChangeEntry[]): string {
  if (changes.length === 0) {
    return "Changed files";
  }

  if (changes.length === 1) {
    return changes[0]?.path ?? "Changed file";
  }

  return `${changes.length} files changed`;
}

export const FileChangeWidget = React.memo(function FileChangeWidget({
  toolCall,
  compact = false,
  className = "",
}: ToolCallWidgetProps) {
  const changes = useMemo(() => parseChanges(toolCall), [toolCall]);
  const isCompleted = toolCall.result != null;

  return (
    <WidgetCard
      className={className}
      compact={compact}
      defaultExpanded={!isCompleted}
      alwaysExpanded={changes.length > 0 && changes.length <= 3}
      header={(
        <WidgetHeader
          icon={<FileEdit size={14} />}
          title={headerTitle(changes)}
          badge={(
            <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
              <Badge variant={isCompleted ? "success" : "accent"} compact>
                {isCompleted ? "applied" : "editing"}
              </Badge>
              {changes.length > 1 && (
                <Badge variant="muted" compact>
                  {changes.length}
                </Badge>
              )}
            </span>
          )}
          compact={compact}
          mono={changes.length === 1}
        />
      )}
    >
      {changes.length === 0 ? (
        <div
          style={{
            fontSize: compact ? 10 : 10.5,
            color: "var(--text-muted)",
            padding: "2px 0",
          }}
        >
          Waiting for file change details.
        </div>
      ) : (
        changes.map((change, index) => (
          <WidgetRow key={`${change.path ?? "change"}-${index}`} compact={compact}>
            <FilePath path={change.path ?? "Unknown file"} maxLength={56} />
            <Badge variant="muted" compact>
              {kindLabel(change.kind)}
            </Badge>
          </WidgetRow>
        ))
      )}
    </WidgetCard>
  );
});
