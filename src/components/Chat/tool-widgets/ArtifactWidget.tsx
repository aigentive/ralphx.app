/**
 * ArtifactWidget — Artifact preview card for get_artifact, get_artifact_version, get_plan_artifact.
 *
 * Header: artifact type badge (SPEC/CODE/etc) + title.
 * Body: first ~3 lines of content with gradient fade, rendered as lightweight markdown preview.
 * Collapsible via WidgetCard.
 */

import React from "react";
import { WidgetCard, Badge } from "./shared";
import type { ToolCall } from "../ToolCallIndicator";

// ============================================================================
// Types
// ============================================================================

interface ParsedArtifact {
  title: string;
  artifactType: string;
  content: string;
  version?: number;
}

interface ArtifactWidgetProps {
  toolCall: ToolCall;
  compact?: boolean;
}

// ============================================================================
// Parsing
// ============================================================================

function parseArtifact(toolCall: ToolCall): ParsedArtifact | null {
  const { name, result } = toolCall;
  if (!result || typeof result !== "object") return null;

  const r = result as Record<string, unknown>;

  // get_artifact / get_plan_artifact: { id, title, artifact_type, content, content_preview, version }
  // get_artifact_version: same shape, with explicit version param in args
  const title =
    (typeof r.title === "string" ? r.title : null) ??
    (typeof r.name === "string" ? r.name : null) ??
    "Untitled Artifact";

  const artifactType =
    (typeof r.artifact_type === "string" ? r.artifact_type : null) ??
    (typeof r.artifactType === "string" ? r.artifactType : null) ??
    "unknown";

  const content =
    (typeof r.content === "string" ? r.content : null) ??
    (typeof r.content_preview === "string" ? r.content_preview : null) ??
    (typeof r.contentPreview === "string" ? r.contentPreview : null) ??
    "";

  // For get_artifact_version, extract version from args
  let version: number | undefined;
  if (name === "get_artifact_version") {
    const args = toolCall.arguments as Record<string, unknown> | undefined;
    if (args && typeof args.version === "number") {
      version = args.version;
    }
  } else if (typeof r.version === "number") {
    version = r.version;
  }

  const base = { title, artifactType, content };
  return version != null ? { ...base, version } : base;
}

// ============================================================================
// Lightweight Markdown Preview
// ============================================================================

/** Render first few lines of markdown as styled preview (headings bold, paragraphs secondary, code mono) */
function MarkdownPreview({ content, compact }: { content: string; compact?: boolean }) {
  const lines = content.split("\n").filter((l) => l.trim() !== "");
  // Show first ~5 non-empty lines for preview
  const previewLines = lines.slice(0, 5);

  return (
    <div className={`${compact ? "text-[10.5px]" : "text-[11.5px]"} leading-[1.55] py-1.5`}>
      {previewLines.map((line, i) => {
        const trimmed = line.trim();

        // H1: # heading
        if (trimmed.startsWith("# ")) {
          return (
            <div
              key={i}
              className={`${compact ? "text-xs" : "text-[13px]"} font-semibold mb-1`}
              style={{ color: "hsl(220 10% 90%)" }}
            >
              {trimmed.replace(/^#+\s*/, "")}
            </div>
          );
        }

        // H2: ## heading
        if (trimmed.startsWith("## ")) {
          return (
            <div
              key={i}
              className={`${compact ? "text-[10.5px]" : "text-[11.5px]"} font-semibold mt-1.5`}
              style={{ color: "hsl(220 10% 90%)" }}
            >
              {trimmed.replace(/^#+\s*/, "")}
            </div>
          );
        }

        // H3+: ### heading
        if (trimmed.startsWith("###")) {
          return (
            <div
              key={i}
              className={`${compact ? "text-[10px]" : "text-[11px]"} font-semibold mt-1`}
              style={{ color: "hsl(220 10% 80%)" }}
            >
              {trimmed.replace(/^#+\s*/, "")}
            </div>
          );
        }

        // Code block markers (``` lines) — skip
        if (trimmed.startsWith("```")) {
          return null;
        }

        // Lines that look like code (indented or inside code blocks)
        if (line.startsWith("    ") || line.startsWith("\t")) {
          return (
            <div
              key={i}
              className={`${compact ? "text-[10px]" : "text-[10.5px]"} mt-0.5`}
              style={{
                color: "hsl(220 10% 55%)",
                fontFamily: "var(--font-mono)",
              }}
            >
              {trimmed}
            </div>
          );
        }

        // Regular paragraph
        return (
          <div
            key={i}
            className={`${compact ? "text-[10px]" : "text-[11px]"} mt-0.5`}
            style={{ color: "hsl(220 10% 60%)" }}
          >
            {trimmed}
          </div>
        );
      })}
    </div>
  );
}

// ============================================================================
// Type Badge
// ============================================================================

const TYPE_LABELS: Record<string, string> = {
  specification: "SPEC",
  research: "RESEARCH",
  design_doc: "DESIGN",
  decision: "DECISION",
  test_plan: "TEST",
  code: "CODE",
};

function artifactTypeLabel(type: string): string {
  const lower = type.toLowerCase();
  return TYPE_LABELS[lower] ?? type.toUpperCase().slice(0, 8);
}

// ============================================================================
// Component
// ============================================================================

export const ArtifactWidget = React.memo(function ArtifactWidget({
  toolCall,
  compact = false,
}: ArtifactWidgetProps) {
  const parsed = parseArtifact(toolCall);

  if (!parsed) return null;

  const header = (
    <>
      <span
        className="text-[9px] px-1.5 py-px rounded font-semibold uppercase tracking-wider flex-shrink-0"
        style={{
          background: "hsl(220 10% 18%)",
          color: "hsl(220 10% 45%)",
          letterSpacing: "0.04em",
        }}
      >
        {artifactTypeLabel(parsed.artifactType)}
      </span>
      <span
        className={`${compact ? "text-[11px]" : "text-[11.5px]"} font-medium flex-1 min-w-0 truncate`}
        style={{ color: "hsl(220 10% 60%)" }}
      >
        {parsed.title}
      </span>
      {parsed.version != null && (
        <Badge variant="muted">v{parsed.version}</Badge>
      )}
    </>
  );

  return (
    <WidgetCard
      header={header}
      compact={compact}
      defaultExpanded={false}
    >
      {parsed.content ? (
        <MarkdownPreview content={parsed.content} compact={compact} />
      ) : (
        <div
          className={`${compact ? "text-[10px]" : "text-[10.5px]"} py-1`}
          style={{ color: "hsl(220 10% 45%)" }}
        >
          No content preview available
        </div>
      )}
    </WidgetCard>
  );
});
