/**
 * ConflictDiffViewer - Renders merged file content with conflict markers highlighted
 *
 * Parses the mergedWithMarkers content to identify conflict sections and renders
 * them with color-coded backgrounds:
 * - Ours (target branch): Red tint
 * - Theirs (source branch): Blue tint
 * - Separator: Gray
 * - Context: Normal text
 */

import { useMemo } from "react";
import type { ConflictDiff } from "@/hooks/useConflictDiff";

interface ConflictDiffViewerProps {
  /** Conflict diff data from useConflictDiff hook */
  conflictDiff: ConflictDiff;
}

type LineType = "context" | "ours" | "theirs" | "separator" | "marker";

interface ParsedLine {
  type: LineType;
  content: string;
  lineNumber: number;
}

/**
 * Parse conflict marker content into classified lines
 *
 * Conflict format:
 * <<<<<<< ours-branch-name
 * content from ours (target branch)
 * =======
 * content from theirs (source branch)
 * >>>>>>> theirs-branch-name
 */
function parseConflictContent(content: string | undefined | null): ParsedLine[] {
  if (!content) return [];
  const lines = content.split("\n");
  const result: ParsedLine[] = [];

  let currentType: LineType = "context";
  let lineNumber = 1;

  for (const line of lines) {
    if (line.startsWith("<<<<<<<")) {
      // Start of conflict - ours section begins
      result.push({ type: "marker", content: line, lineNumber });
      currentType = "ours";
    } else if (line.startsWith("=======")) {
      // Separator between ours and theirs
      result.push({ type: "separator", content: line, lineNumber });
      currentType = "theirs";
    } else if (line.startsWith(">>>>>>>")) {
      // End of conflict - back to context
      result.push({ type: "marker", content: line, lineNumber });
      currentType = "context";
    } else {
      // Regular content line
      result.push({ type: currentType, content: line, lineNumber });
    }
    lineNumber++;
  }

  return result;
}

/**
 * Get line background color based on type
 */
function getLineBackground(type: LineType): string {
  switch (type) {
    case "ours":
      return "rgba(255, 69, 58, 0.15)"; // Red tint
    case "theirs":
      return "rgba(64, 156, 255, 0.15)"; // Blue tint
    case "separator":
      return "rgba(255, 255, 255, 0.08)"; // Gray
    case "marker":
      return "rgba(255, 255, 255, 0.04)"; // Subtle gray for markers
    default:
      return "transparent";
  }
}

/**
 * Get line text color based on type
 */
function getLineTextColor(type: LineType): string {
  switch (type) {
    case "ours":
    case "theirs":
    case "context":
      return "hsl(220 10% 70%)";
    case "separator":
      return "hsl(220 10% 50%)";
    case "marker":
      return "hsl(220 10% 40%)";
    default:
      return "hsl(220 10% 70%)";
  }
}

/**
 * Get prefix character and color for line type
 */
function getLinePrefix(type: LineType): { char: string; color: string } {
  switch (type) {
    case "ours":
      return { char: "-", color: "#ff453a" }; // Red
    case "theirs":
      return { char: "+", color: "#409cff" }; // Blue
    case "separator":
      return { char: "=", color: "hsl(220 10% 50%)" };
    case "marker":
      return { char: "", color: "transparent" };
    default:
      return { char: " ", color: "transparent" };
  }
}

/**
 * Get file extension for language badge display
 */
function getFileExtension(filePath: string): string {
  const parts = filePath.split(".");
  if (parts.length > 1) {
    return parts[parts.length - 1] ?? "";
  }
  return "";
}

/**
 * Render a single line
 */
function renderLine(line: ParsedLine, index: number): React.ReactNode {
  const { char, color: prefixColor } = getLinePrefix(line.type);

  return (
    <div
      key={index}
      className="flex"
      style={{
        backgroundColor: getLineBackground(line.type),
        minHeight: "20px",
      }}
    >
      {/* Line number */}
      <div
        className="w-12 shrink-0 text-right pr-2 select-none"
        style={{
          position: "sticky",
          left: 0,
          color: "hsl(220 10% 35%)",
          backgroundColor: "hsl(220 10% 10%)",
        }}
      >
        {line.lineNumber}
      </div>

      {/* Prefix */}
      <div
        className="w-6 shrink-0 text-center select-none font-bold"
        style={{
          position: "sticky",
          left: 48,
          color: prefixColor,
          backgroundColor: "hsl(220 10% 10%)",
        }}
      >
        {char}
      </div>

      {/* Content */}
      <div
        className="flex-1 pr-4 min-w-0 whitespace-pre"
        style={{
          color: getLineTextColor(line.type),
        }}
      >
        {line.content || " "}
      </div>
    </div>
  );
}

export function ConflictDiffViewer({ conflictDiff }: ConflictDiffViewerProps) {
  const { filePath, mergedWithMarkers, language } = conflictDiff;

  const parsedLines = useMemo(
    () => parseConflictContent(mergedWithMarkers),
    [mergedWithMarkers]
  );

  const displayLanguage = language || getFileExtension(filePath);

  return (
    <div className="h-full overflow-y-auto">
      <div
        className="font-mono text-[13px] leading-[20px]"
        style={{ backgroundColor: "hsl(220 10% 8%)" }}
      >
        {/* Header with file path and language badge */}
        <div
          className="flex items-center justify-between px-3 py-2 border-b"
          style={{ borderColor: "rgba(255,255,255,0.06)" }}
        >
          <span
            className="text-sm truncate"
            style={{ color: "hsl(220 10% 80%)" }}
          >
            {filePath}
          </span>
          {displayLanguage && (
            <span
              className="text-[11px] px-2 py-0.5 rounded ml-2 shrink-0"
              style={{
                backgroundColor: "rgba(255,255,255,0.08)",
                color: "hsl(220 10% 60%)",
              }}
            >
              {displayLanguage}
            </span>
          )}
        </div>

        {/* Conflict legend */}
        <div
          className="flex items-center gap-4 px-3 py-1.5 text-[11px]"
          style={{
            backgroundColor: "rgba(255,255,255,0.02)",
            borderBottom: "1px solid rgba(255,255,255,0.04)",
          }}
        >
          <span className="flex items-center gap-1.5">
            <span
              className="w-3 h-3 rounded"
              style={{ backgroundColor: "rgba(255, 69, 58, 0.15)" }}
            />
            <span style={{ color: "#ff453a" }}>-</span>
            <span style={{ color: "hsl(220 10% 50%)" }}>Ours (current)</span>
          </span>
          <span className="flex items-center gap-1.5">
            <span
              className="w-3 h-3 rounded"
              style={{ backgroundColor: "rgba(64, 156, 255, 0.15)" }}
            />
            <span style={{ color: "#409cff" }}>+</span>
            <span style={{ color: "hsl(220 10% 50%)" }}>Theirs (incoming)</span>
          </span>
        </div>

        {/* Diff content */}
        <div className="overflow-x-auto">
          {parsedLines.map((line, index) => renderLine(line, index))}
        </div>
      </div>
    </div>
  );
}
