/**
 * TeamResearchView - Full-content artifact cards for the "Team Research" tab
 *
 * Fetches full artifact content on mount and renders each as a scrollable
 * glass-morphism card with ReactMarkdown. macOS Tahoe styling, warm orange accent.
 */

import { useState, useEffect } from "react";
import { Microscope, BarChart3, FileText, Loader2 } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { artifactApi } from "@/api/artifact";
import type { Artifact } from "@/types/artifact";
import type { TeamArtifactSummary } from "@/api/team";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

export interface TeamResearchViewProps {
  artifacts: TeamArtifactSummary[];
  sessionId: string;
}

interface ArtifactCardState {
  full: Artifact | null;
  loading: boolean;
  error: string | null;
}

// ============================================================================
// Icon Config (matches TeamArtifactChips pattern)
// ============================================================================

interface ArtifactTypeConfig {
  icon: React.ElementType;
  color: string;
}

const DEFAULT_CONFIG: ArtifactTypeConfig = { icon: FileText, color: "hsl(220 10% 50%)" };

const ARTIFACT_TYPE_CONFIG: Record<string, ArtifactTypeConfig> = {
  research: { icon: Microscope, color: "hsl(14 100% 60%)" },
  analysis: { icon: BarChart3, color: "hsl(174 60% 50%)" },
  summary: DEFAULT_CONFIG,
};

function getTypeConfig(artifactType: string): ArtifactTypeConfig {
  return ARTIFACT_TYPE_CONFIG[artifactType.toLowerCase()] ?? DEFAULT_CONFIG;
}

// ============================================================================
// Markdown Components (dark theme, matches PlanDisplay)
// ============================================================================

const markdownComponents = {
  a: ({ href, children, ...props }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="text-accent-primary hover:text-accent-primary/80 underline underline-offset-2 decoration-accent-primary/30 hover:decoration-accent-primary/60 transition-colors"
      {...props}
    >
      {children}
    </a>
  ),
  code: ({ className, children, ...props }: React.HTMLAttributes<HTMLElement>) => {
    const isBlock = className?.includes("language-");
    if (isBlock) {
      return (
        <code
          className={cn(
            "block p-3 rounded-md text-[13px] font-mono overflow-x-auto",
            "bg-white/[0.02] border border-white/[0.04]",
            className
          )}
          {...props}
        >
          {children}
        </code>
      );
    }
    return (
      <code
        className="px-1.5 py-0.5 rounded text-[13px] font-mono bg-white/[0.04] text-text-primary"
        {...props}
      >
        {children}
      </code>
    );
  },
  pre: ({ children, ...props }: React.HTMLAttributes<HTMLPreElement>) => (
    <pre className="my-3 rounded-md overflow-hidden" {...props}>
      {children}
    </pre>
  ),
  p: ({ children, ...props }: React.HTMLAttributes<HTMLParagraphElement>) => (
    <p className="mb-3 last:mb-0 leading-relaxed text-text-secondary" {...props}>
      {children}
    </p>
  ),
  ul: ({ children, ...props }: React.HTMLAttributes<HTMLUListElement>) => (
    <ul className="mb-3 space-y-1.5 pl-4" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }: React.HTMLAttributes<HTMLOListElement>) => (
    <ol className="list-decimal mb-3 space-y-1.5 pl-4" {...props}>
      {children}
    </ol>
  ),
  li: ({ children, ...props }: React.LiHTMLAttributes<HTMLLIElement>) => (
    <li className="text-text-secondary leading-relaxed relative before:content-['•'] before:absolute before:-left-3 before:text-accent-primary/50" {...props}>
      {children}
    </li>
  ),
  h1: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h1 className="text-lg font-medium text-text-primary mb-3 mt-6 first:mt-0 pb-2 border-b border-white/[0.06]" {...props}>
      {children}
    </h1>
  ),
  h2: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h2 className="text-base font-medium text-text-primary mb-2 mt-5" {...props}>
      {children}
    </h2>
  ),
  h3: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h3 className="text-sm font-medium text-text-primary mb-2 mt-4" {...props}>
      {children}
    </h3>
  ),
  blockquote: ({ children, ...props }: React.HTMLAttributes<HTMLQuoteElement>) => (
    <blockquote
      className="border-l-2 border-accent-primary/30 pl-4 my-3 text-text-muted italic"
      {...props}
    >
      {children}
    </blockquote>
  ),
  hr: ({ ...props }: React.HTMLAttributes<HTMLHRElement>) => (
    <hr className="my-6 border-white/[0.06]" {...props} />
  ),
  table: ({ children, ...props }: React.TableHTMLAttributes<HTMLTableElement>) => (
    <div className="my-3 overflow-x-auto rounded-lg border border-white/[0.06]">
      <table className="w-full text-sm border-collapse" {...props}>
        {children}
      </table>
    </div>
  ),
  thead: ({ children, ...props }: React.HTMLAttributes<HTMLTableSectionElement>) => (
    <thead className="bg-white/[0.02]" {...props}>
      {children}
    </thead>
  ),
  tbody: ({ children, ...props }: React.HTMLAttributes<HTMLTableSectionElement>) => (
    <tbody {...props}>{children}</tbody>
  ),
  tr: ({ children, ...props }: React.HTMLAttributes<HTMLTableRowElement>) => (
    <tr className="border-b border-white/[0.06] last:border-b-0" {...props}>
      {children}
    </tr>
  ),
  th: ({ children, ...props }: React.ThHTMLAttributes<HTMLTableCellElement>) => (
    <th className="px-3 py-2 text-left text-xs font-medium text-text-primary uppercase tracking-wider" {...props}>
      {children}
    </th>
  ),
  td: ({ children, ...props }: React.TdHTMLAttributes<HTMLTableCellElement>) => (
    <td className="px-3 py-2 text-text-secondary" {...props}>
      {children}
    </td>
  ),
};

// ============================================================================
// Component
// ============================================================================

export function TeamResearchView({ artifacts }: TeamResearchViewProps) {
  const [cardStates, setCardStates] = useState<Record<string, ArtifactCardState>>({});

  // Fetch full content for all artifacts on mount / when artifacts change
  useEffect(() => {
    if (artifacts.length === 0) return;

    let cancelled = false;

    // Initialize loading states
    setCardStates((prev) => {
      const next = { ...prev };
      for (const a of artifacts) {
        if (!next[a.id]) {
          next[a.id] = { full: null, loading: true, error: null };
        }
      }
      return next;
    });

    // Fetch each artifact's full content
    for (const artifact of artifacts) {
      artifactApi.get(artifact.id)
        .then((full) => {
          if (cancelled) return;
          setCardStates((prev) => ({
            ...prev,
            [artifact.id]: { full, loading: false, error: null },
          }));
        })
        .catch((err) => {
          if (cancelled) return;
          console.error(`Failed to fetch artifact ${artifact.id}:`, err);
          setCardStates((prev) => ({
            ...prev,
            [artifact.id]: { full: null, loading: false, error: "Failed to load content" },
          }));
        });
    }

    return () => { cancelled = true; };
  }, [artifacts]);

  // Empty state
  if (artifacts.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16">
        <FileText className="w-10 h-10 mb-3" style={{ color: "hsl(220 10% 30%)" }} />
        <span className="text-[13px] font-medium" style={{ color: "hsl(220 10% 45%)" }}>
          No team research artifacts yet
        </span>
        <span className="text-[11px] mt-1" style={{ color: "hsl(220 10% 35%)" }}>
          Artifacts will appear here once team research completes
        </span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Section header */}
      <div className="flex items-center gap-2">
        <span
          className="text-[11px] font-medium tracking-wide uppercase"
          style={{ color: "hsl(220 10% 50%)" }}
        >
          Team Research
        </span>
        <span
          className="text-[10px] font-medium px-1.5 py-0.5 rounded-md"
          style={{
            background: "hsla(220 10% 100% / 0.04)",
            border: "1px solid hsla(220 10% 100% / 0.06)",
            color: "hsl(220 10% 50%)",
          }}
        >
          {artifacts.length} artifact{artifacts.length !== 1 ? "s" : ""}
        </span>
      </div>

      {/* Artifact cards */}
      {artifacts.map((artifact) => {
        const config = getTypeConfig(artifact.artifact_type);
        const Icon = config.icon;
        const state = cardStates[artifact.id];
        const isLoading = !state || state.loading;
        const fullArtifact = state?.full;
        const fullContent = fullArtifact?.content.type === "inline"
          ? fullArtifact.content.text
          : null;
        const author = fullArtifact?.metadata.createdBy;

        return (
          <div
            key={artifact.id}
            className="rounded-xl transition-all duration-200"
            style={{
              background: "hsla(220 10% 100% / 0.02)",
              border: "1px solid hsla(220 10% 100% / 0.06)",
            }}
          >
            {/* Card header */}
            <div className="flex items-center gap-3 px-4 py-3">
              <div
                className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
                style={{
                  background: `${config.color}15`,
                  border: `1px solid ${config.color}30`,
                }}
              >
                <Icon
                  className="w-4 h-4"
                  style={{ color: config.color }}
                />
              </div>

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span
                    className="text-[13px] font-medium truncate tracking-[-0.01em]"
                    style={{ color: "hsl(220 10% 90%)" }}
                  >
                    {artifact.name}
                  </span>
                  <span
                    className="text-[10px] font-medium px-1.5 py-0.5 rounded-md flex-shrink-0"
                    style={{
                      background: "hsla(220 10% 100% / 0.04)",
                      border: "1px solid hsla(220 10% 100% / 0.06)",
                      color: "hsl(220 10% 50%)",
                    }}
                  >
                    v{artifact.version}
                  </span>
                </div>
                {author && (
                  <span
                    className="text-[11px] mt-0.5 block"
                    style={{ color: "hsl(220 10% 50%)" }}
                  >
                    by {author}
                  </span>
                )}
              </div>
            </div>

            {/* Divider */}
            <div
              className="mx-4"
              style={{ borderTop: "1px solid hsla(220 10% 100% / 0.06)" }}
            />

            {/* Content area */}
            <div className="px-4 py-3">
              {isLoading ? (
                <div className="flex flex-col gap-2">
                  {/* Show preview while loading */}
                  <p
                    className="text-[13px] leading-relaxed"
                    style={{ color: "hsl(220 10% 55%)" }}
                  >
                    {artifact.content_preview}
                  </p>
                  <div className="flex items-center gap-2 pt-1">
                    <Loader2
                      className="w-3.5 h-3.5 animate-spin"
                      style={{ color: "hsl(14 100% 60%)" }}
                    />
                    <span
                      className="text-[11px]"
                      style={{ color: "hsl(220 10% 45%)" }}
                    >
                      Loading full content...
                    </span>
                  </div>
                </div>
              ) : state?.error ? (
                <div className="py-4 text-center">
                  <span
                    className="text-[12px]"
                    style={{ color: "hsl(0 70% 60%)" }}
                  >
                    {state.error}
                  </span>
                  <p
                    className="text-[13px] leading-relaxed mt-2"
                    style={{ color: "hsl(220 10% 55%)" }}
                  >
                    {artifact.content_preview}
                  </p>
                </div>
              ) : fullContent ? (
                <div className="text-[13px] leading-relaxed">
                  <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                    {fullContent}
                  </ReactMarkdown>
                </div>
              ) : (
                <p
                  className="text-[13px] italic py-4 text-center"
                  style={{ color: "hsl(220 10% 50%)" }}
                >
                  No content available
                </p>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
