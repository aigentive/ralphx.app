/**
 * ActivityMessage - Individual message display component
 */

import { useCallback } from "react";
import { ChevronDown, Copy, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { UnifiedActivityMessage } from "./ActivityView.types";
import {
  getMessageIcon,
  getMessageColor,
  getMessageBgColor,
  getToolName,
  formatTimestamp,
  highlightJSON,
} from "./ActivityView.utils";

export interface ActivityMessageProps {
  message: UnifiedActivityMessage;
  isExpanded: boolean;
  onToggle: () => void;
  copied: boolean;
  onCopy: () => void;
}

export function ActivityMessage({
  message,
  isExpanded,
  onToggle,
  copied,
  onCopy,
}: ActivityMessageProps) {
  const { type, content, timestamp, metadata, internalStatus } = message;
  const hasDetails = type === "tool_call" || type === "tool_result" || metadata;
  const toolName = getToolName(content);

  // Parse content for display
  const displayContent = content.length > 200 && !isExpanded
    ? content.slice(0, 200) + "..."
    : content;

  const handleCopy = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (metadata) {
        try {
          navigator.clipboard.writeText(JSON.stringify(metadata, null, 2));
          onCopy();
        } catch {
          // Silently fail if metadata can't be stringified (shouldn't happen, but safe)
        }
      }
    },
    [metadata, onCopy]
  );

  return (
    <div
      data-testid="activity-message"
      data-type={type}
      className="rounded-lg transition-all"
      style={{
        backgroundColor: getMessageBgColor(type),
        borderLeft: `3px solid ${getMessageColor(type)}`,
      }}
    >
      {/* Header */}
      <div
        className={cn(
          "flex items-start gap-3 px-3 py-2.5 select-none",
          hasDetails && "cursor-pointer hover:bg-white/[0.02]"
        )}
        onClick={hasDetails ? onToggle : undefined}
      >
        {/* Expand/Collapse Icon */}
        {hasDetails && (
          <ChevronDown
            className={cn(
              "w-3 h-3 mt-1 text-[var(--text-muted)] transition-transform shrink-0",
              !isExpanded && "-rotate-90"
            )}
          />
        )}
        {!hasDetails && <span className="w-3 shrink-0" />}

        {/* Type Icon */}
        <span className="mt-0.5 shrink-0" style={{ color: getMessageColor(type) }}>
          {getMessageIcon(type)}
        </span>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            {toolName && (
              <span
                className="text-xs font-mono px-1.5 py-0.5 rounded bg-[var(--bg-base)]"
                style={{ color: getMessageColor(type) }}
              >
                {toolName}
              </span>
            )}
            <span className="text-xs text-[var(--text-muted)] capitalize">
              {type.replace("_", " ")}
            </span>
            {internalStatus && (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--bg-base)] text-[var(--text-muted)]">
                {internalStatus}
              </span>
            )}
          </div>
          <p className="text-sm text-[var(--text-primary)] whitespace-pre-wrap break-words">
            {displayContent}
          </p>
        </div>

        {/* Timestamp */}
        <span className="text-xs text-[var(--text-muted)] shrink-0 ml-2">
          {formatTimestamp(timestamp)}
        </span>
      </div>

      {/* Expanded Details */}
      {hasDetails && isExpanded && metadata && (
        <div className="ml-9 mr-3 pb-3 border-t border-[var(--border-subtle)]">
          <div className="pt-3 relative">
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-medium text-[var(--text-muted)]">Details</span>
              <Button
                variant="ghost"
                size="icon"
                className="h-6 w-6 hover:bg-[var(--bg-hover)]"
                onClick={handleCopy}
              >
                {copied ? (
                  <Check className="w-3.5 h-3.5 text-[var(--status-success)]" />
                ) : (
                  <Copy className="w-3.5 h-3.5 text-[var(--text-muted)]" />
                )}
              </Button>
            </div>
            <pre className="text-xs font-mono p-3 rounded-md bg-[var(--bg-base)] text-[var(--text-secondary)] overflow-x-auto max-h-[300px] overflow-y-auto">
              {(() => {
                try {
                  return highlightJSON(JSON.stringify(metadata, null, 2));
                } catch {
                  // Fallback to string representation if stringify fails
                  return String(metadata);
                }
              })()}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
