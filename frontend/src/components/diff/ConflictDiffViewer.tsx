/**
 * ConflictDiffViewer - Chunk-based conflict diff using SimpleDiffView
 *
 * Renders ours vs theirs content as a unified diff with:
 * - Deletions (red) = lines only in ours (current branch)
 * - Additions (blue) = lines only in theirs (incoming branch)
 * - Context = shared between both
 */

import type { ConflictDiff } from "@/hooks/useConflictDiff";
import { SimpleDiffView } from "./SimpleDiffView";

interface ConflictDiffViewerProps {
  /** Conflict diff data from useConflictDiff hook */
  conflictDiff: ConflictDiff;
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

export function ConflictDiffViewer({ conflictDiff }: ConflictDiffViewerProps) {
  const { filePath, oursContent, theirsContent, language } = conflictDiff;

  const displayLanguage = language || getFileExtension(filePath);

  return (
    <div className="h-full flex flex-col">
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
              style={{ backgroundColor: "rgba(255, 69, 58, 0.12)" }}
            />
            <span style={{ color: "#ff453a" }}>-</span>
            <span style={{ color: "hsl(220 10% 50%)" }}>Ours (current)</span>
          </span>
          <span className="flex items-center gap-1.5">
            <span
              className="w-3 h-3 rounded"
              style={{ backgroundColor: "rgba(64, 156, 255, 0.12)" }}
            />
            <span style={{ color: "#409cff" }}>+</span>
            <span style={{ color: "hsl(220 10% 50%)" }}>Theirs (incoming)</span>
          </span>
        </div>
      </div>

      {/* Diff content via SimpleDiffView */}
      <div className="flex-1 min-h-0">
        <SimpleDiffView
          oldContent={oursContent ?? ""}
          newContent={theirsContent ?? ""}
          language={displayLanguage}
          variant="conflict"
        />
      </div>
    </div>
  );
}
