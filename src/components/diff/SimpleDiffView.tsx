/**
 * SimpleDiffView - A reliable inline diff renderer
 *
 * Renders unified diff content with line numbers and syntax highlighting.
 * Fallback for when @git-diff-view/react has issues.
 */

import { useMemo } from "react";
import { ScrollArea } from "@/components/ui/scroll-area";

interface SimpleDiffViewProps {
  oldContent: string;
  newContent: string;
  language?: string | undefined;
}

interface DiffLine {
  type: "context" | "addition" | "deletion" | "header";
  content: string;
  oldLineNum: number | null;
  newLineNum: number | null;
}

/**
 * Simple diff algorithm - generates unified diff lines
 */
function computeDiff(oldContent: string, newContent: string): DiffLine[] {
  const oldLines = oldContent.split("\n");
  const newLines = newContent.split("\n");
  const result: DiffLine[] = [];

  // Simple LCS-based diff
  const lcs = computeLCS(oldLines, newLines);

  let oldIdx = 0;
  let newIdx = 0;
  let oldLineNum = 1;
  let newLineNum = 1;

  for (const match of lcs) {
    // Add deletions (lines in old but not in new before this match)
    while (oldIdx < match.oldIdx) {
      result.push({
        type: "deletion",
        content: oldLines[oldIdx] ?? "",
        oldLineNum: oldLineNum++,
        newLineNum: null,
      });
      oldIdx++;
    }

    // Add additions (lines in new but not in old before this match)
    while (newIdx < match.newIdx) {
      result.push({
        type: "addition",
        content: newLines[newIdx] ?? "",
        oldLineNum: null,
        newLineNum: newLineNum++,
      });
      newIdx++;
    }

    // Add context line (matching)
    result.push({
      type: "context",
      content: oldLines[oldIdx] ?? "",
      oldLineNum: oldLineNum++,
      newLineNum: newLineNum++,
    });
    oldIdx++;
    newIdx++;
  }

  // Add remaining deletions
  while (oldIdx < oldLines.length) {
    result.push({
      type: "deletion",
      content: oldLines[oldIdx] ?? "",
      oldLineNum: oldLineNum++,
      newLineNum: null,
    });
    oldIdx++;
  }

  // Add remaining additions
  while (newIdx < newLines.length) {
    result.push({
      type: "addition",
      content: newLines[newIdx] ?? "",
      oldLineNum: null,
      newLineNum: newLineNum++,
    });
    newIdx++;
  }

  return result;
}

interface Match {
  oldIdx: number;
  newIdx: number;
}

/**
 * Compute Longest Common Subsequence indices
 */
function computeLCS(oldLines: string[], newLines: string[]): Match[] {
  const m = oldLines.length;
  const n = newLines.length;

  // Build DP table
  const dp: number[][] = Array(m + 1)
    .fill(null)
    .map(() => Array(n + 1).fill(0));

  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      if (oldLines[i - 1] === newLines[j - 1]) {
        dp[i]![j] = (dp[i - 1]?.[j - 1] ?? 0) + 1;
      } else {
        dp[i]![j] = Math.max(dp[i - 1]?.[j] ?? 0, dp[i]?.[j - 1] ?? 0);
      }
    }
  }

  // Backtrack to find matches
  const matches: Match[] = [];
  let i = m;
  let j = n;

  while (i > 0 && j > 0) {
    if (oldLines[i - 1] === newLines[j - 1]) {
      matches.unshift({ oldIdx: i - 1, newIdx: j - 1 });
      i--;
      j--;
    } else if ((dp[i - 1]?.[j] ?? 0) > (dp[i]?.[j - 1] ?? 0)) {
      i--;
    } else {
      j--;
    }
  }

  return matches;
}

/**
 * Get line background color based on type
 */
function getLineBackground(type: DiffLine["type"]): string {
  switch (type) {
    case "addition":
      return "rgba(52, 199, 89, 0.12)";
    case "deletion":
      return "rgba(255, 69, 58, 0.12)";
    default:
      return "transparent";
  }
}

/**
 * Get line number color based on type
 */
function getLineNumColor(type: DiffLine["type"]): string {
  switch (type) {
    case "addition":
      return "rgba(52, 199, 89, 0.6)";
    case "deletion":
      return "rgba(255, 69, 58, 0.6)";
    default:
      return "hsl(220 10% 35%)";
  }
}

/**
 * Get prefix character for line type
 */
function getLinePrefix(type: DiffLine["type"]): string {
  switch (type) {
    case "addition":
      return "+";
    case "deletion":
      return "-";
    default:
      return " ";
  }
}

/**
 * Get prefix color
 */
function getPrefixColor(type: DiffLine["type"]): string {
  switch (type) {
    case "addition":
      return "#34c759";
    case "deletion":
      return "#ff453a";
    default:
      return "transparent";
  }
}

export function SimpleDiffView({ oldContent, newContent }: SimpleDiffViewProps) {
  const diffLines = useMemo(
    () => computeDiff(oldContent, newContent),
    [oldContent, newContent]
  );

  if (diffLines.length === 0) {
    return (
      <div
        className="flex items-center justify-center h-full"
        style={{ color: "hsl(220 10% 50%)" }}
      >
        <p className="text-sm">No changes</p>
      </div>
    );
  }

  return (
    <ScrollArea className="h-full">
      <div
        className="font-mono text-[13px] leading-[20px]"
        style={{ backgroundColor: "hsl(220 10% 8%)" }}
      >
        {diffLines.map((line, index) => (
          <div
            key={index}
            className="flex"
            style={{
              backgroundColor: getLineBackground(line.type),
              minHeight: "20px",
            }}
          >
            {/* Old line number */}
            <div
              className="w-12 shrink-0 text-right pr-2 select-none"
              style={{
                color: getLineNumColor(line.type),
                backgroundColor: "hsl(220 10% 10%)",
              }}
            >
              {line.oldLineNum ?? ""}
            </div>

            {/* New line number */}
            <div
              className="w-12 shrink-0 text-right pr-2 select-none border-r"
              style={{
                color: getLineNumColor(line.type),
                backgroundColor: "hsl(220 10% 10%)",
                borderColor: "hsl(220 10% 15%)",
              }}
            >
              {line.newLineNum ?? ""}
            </div>

            {/* Prefix (+/-/ ) */}
            <div
              className="w-6 shrink-0 text-center select-none font-bold"
              style={{ color: getPrefixColor(line.type) }}
            >
              {getLinePrefix(line.type)}
            </div>

            {/* Content */}
            <div
              className="flex-1 pr-4 whitespace-pre overflow-x-auto"
              style={{
                color:
                  line.type === "deletion"
                    ? "hsl(220 10% 60%)"
                    : "hsl(220 10% 80%)",
              }}
            >
              {line.content || " "}
            </div>
          </div>
        ))}
      </div>
    </ScrollArea>
  );
}
