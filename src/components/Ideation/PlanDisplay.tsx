/**
 * PlanDisplay - macOS Tahoe styled plan artifact display
 *
 * Design: Glass-morphism collapsible with refined typography,
 * warm orange accent for actions, and smooth animations.
 */

import { useState, useCallback, useEffect } from "react";
import { FileEdit, Download, CheckCircle2, ChevronDown, FileText, Sparkles, History, Loader2, ArrowLeft } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { artifactApi } from "@/api/artifact";
import type { Artifact } from "@/types/artifact";
import { cn } from "@/lib/utils";

// ============================================================================
// Types
// ============================================================================

export interface PlanDisplayProps {
  plan: Artifact;
  showApprove?: boolean;
  linkedProposalsCount?: number;
  onEdit?: () => void;
  onExport?: () => void;
  onApprove?: () => void;
  isApproved?: boolean;
  /** Controlled expanded state - if provided, component is controlled */
  isExpanded?: boolean;
  /** Callback when expanded state changes */
  onExpandedChange?: (expanded: boolean) => void;
  /** @deprecated No longer used - version selection is now inline. Will be removed in Task 2. */
  onViewHistory?: () => void;
}

// ============================================================================
// Markdown Components - Minimal styling
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
  // Table support (GFM)
  table: ({ children, ...props }: React.TableHTMLAttributes<HTMLTableElement>) => (
    <div className="my-3 overflow-x-auto rounded-lg border border-white/[0.06]">
      <table
        className="w-full text-sm border-collapse"
        {...props}
      >
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
    <tr
      className="border-b border-white/[0.06] last:border-b-0"
      {...props}
    >
      {children}
    </tr>
  ),
  th: ({ children, ...props }: React.ThHTMLAttributes<HTMLTableCellElement>) => (
    <th
      className="px-3 py-2 text-left text-xs font-medium text-text-primary uppercase tracking-wider"
      {...props}
    >
      {children}
    </th>
  ),
  td: ({ children, ...props }: React.TdHTMLAttributes<HTMLTableCellElement>) => (
    <td
      className="px-3 py-2 text-text-secondary"
      {...props}
    >
      {children}
    </td>
  ),
};

// ============================================================================
// Component
// ============================================================================

export function PlanDisplay({
  plan,
  showApprove = false,
  linkedProposalsCount = 0,
  onEdit,
  onExport,
  onApprove,
  isApproved = false,
  isExpanded,
  onExpandedChange,
}: PlanDisplayProps) {
  const [isHovered, setIsHovered] = useState(false);
  // Use controlled state if isExpanded prop is provided, otherwise use internal state
  // Default to collapsed (false) for initial render
  const [internalIsOpen, setInternalIsOpen] = useState(false);
  const isOpen = isExpanded !== undefined ? isExpanded : internalIsOpen;
  const setIsOpen = onExpandedChange ?? setInternalIsOpen;

  // Version selector state
  const [selectedVersion, setSelectedVersion] = useState(plan.metadata.version);
  const [historicalContent, setHistoricalContent] = useState<string | null>(null);
  const [isLoadingVersion, setIsLoadingVersion] = useState(false);
  const [isVersionDropdownOpen, setIsVersionDropdownOpen] = useState(false);

  // Reset to latest when plan changes (new artifact or version update)
  useEffect(() => {
    setSelectedVersion(plan.metadata.version);
    setHistoricalContent(null);
  }, [plan.id, plan.metadata.version]);

  // Fetch historical version when selection changes
  useEffect(() => {
    if (selectedVersion === plan.metadata.version) {
      setHistoricalContent(null);
      return;
    }

    let cancelled = false;
    setIsLoadingVersion(true);

    artifactApi.getAtVersion(plan.id, selectedVersion)
      .then((artifact) => {
        if (cancelled) return;
        if (artifact?.content.type === "inline") {
          setHistoricalContent(artifact.content.text);
        } else {
          setHistoricalContent(null);
        }
      })
      .catch((err) => {
        if (cancelled) return;
        console.error("Failed to fetch historical version:", err);
        setHistoricalContent(null);
      })
      .finally(() => {
        if (!cancelled) setIsLoadingVersion(false);
      });

    return () => { cancelled = true; };
  }, [plan.id, selectedVersion, plan.metadata.version]);

  const planContent = plan.content.type === "inline" ? plan.content.text : "";
  const displayContent = historicalContent ?? planContent;
  const isViewingHistorical = selectedVersion !== plan.metadata.version;

  const handleBackToLatest = useCallback(() => {
    setSelectedVersion(plan.metadata.version);
  }, [plan.metadata.version]);

  const handleVersionSelect = useCallback((version: number) => {
    setSelectedVersion(version);
    // Auto-expand when selecting a version
    if (!isOpen) {
      setIsOpen(true);
    }
  }, [isOpen, setIsOpen]);

  const handleExport = useCallback(() => {
    if (onExport) {
      onExport();
      return;
    }

    const blob = new Blob([planContent], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${plan.name.replace(/[^a-z0-9]/gi, "_").toLowerCase()}.md`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [planContent, plan.name, onExport]);

  return (
    <div
      className="group"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        {/* Header - flat Tahoe style */}
        <div
          className="rounded-xl transition-all duration-200"
          style={{
            padding: "12px 14px",
            background: isOpen
              ? "hsla(14 100% 60% / 0.08)"
              : isHovered
                ? "hsla(220 10% 100% / 0.03)"
                : "hsla(220 10% 100% / 0.02)",
            border: isOpen
              ? "1px solid hsla(14 100% 60% / 0.2)"
              : "1px solid hsla(220 10% 100% / 0.06)",
          }}
        >
          <div className="flex items-center gap-3">
            <CollapsibleTrigger asChild>
              <button
                className="flex items-center gap-3 text-left flex-1 min-w-0"
              >
                {/* Icon - flat style */}
                <div
                  className="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0 transition-all duration-200"
                  style={{
                    background: isOpen
                      ? "hsla(14 100% 60% / 0.15)"
                      : "hsla(220 10% 100% / 0.04)",
                    border: isOpen
                      ? "1px solid hsla(14 100% 60% / 0.25)"
                      : "1px solid hsla(220 10% 100% / 0.06)",
                  }}
                >
                  <FileText
                    className="w-4 h-4 transition-colors duration-200"
                    style={{ color: isOpen ? "hsl(14 100% 60%)" : "hsl(220 10% 50%)" }}
                  />
                </div>

                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span
                      className="text-[13px] font-medium truncate tracking-[-0.01em]"
                      style={{ color: "hsl(220 10% 90%)" }}
                    >
                      {plan.name}
                    </span>

                    <span
                      className="text-[10px] font-medium px-1.5 py-0.5 rounded-md flex-shrink-0"
                      style={{
                        background: "hsla(220 10% 100% / 0.04)",
                        border: "1px solid hsla(220 10% 100% / 0.06)",
                        color: "hsl(220 10% 50%)",
                      }}
                    >
                      v{plan.metadata.version}
                    </span>
                  </div>

                  {linkedProposalsCount > 0 && (
                    <span
                      className="text-[11px] mt-0.5 block"
                      style={{ color: "hsl(220 10% 50%)" }}
                    >
                      {linkedProposalsCount} linked proposal{linkedProposalsCount !== 1 ? "s" : ""}
                    </span>
                  )}
                </div>

                <ChevronDown
                  className={cn(
                    "w-4 h-4 transition-transform duration-200 flex-shrink-0",
                    !isOpen && "-rotate-90"
                  )}
                  style={{ color: "hsl(220 10% 50%)" }}
                />
              </button>
            </CollapsibleTrigger>

            {/* Actions - appear on hover, stay visible when dropdown is open */}
            <div className={cn(
              "flex items-center gap-1 transition-opacity duration-150",
              isVersionDropdownOpen || isHovered ? "opacity-100" : "opacity-0"
            )}>
              {showApprove && !isApproved && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={onApprove}
                  className="h-7 px-2.5 text-[11px] font-semibold gap-1.5 rounded-lg transition-colors duration-150"
                  style={{
                    color: "hsl(14 100% 60%)",
                    background: "hsla(14 100% 60% / 0.1)",
                    border: "1px solid hsla(14 100% 60% / 0.2)",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "hsla(14 100% 60% / 0.15)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "hsla(14 100% 60% / 0.1)";
                  }}
                >
                  <Sparkles className="w-3 h-3" />
                  Approve
                </Button>
              )}

              {isApproved && (
                <span
                  className="flex items-center gap-1.5 text-[11px] font-medium px-2.5 py-1 rounded-lg"
                  style={{
                    background: "hsla(145 70% 45% / 0.1)",
                    border: "1px solid hsla(145 70% 45% / 0.2)",
                    color: "hsl(145 70% 45%)",
                  }}
                >
                  <CheckCircle2 className="w-3 h-3" />
                  Approved
                </span>
              )}

              {plan.metadata.version > 1 && (
                <DropdownMenu open={isVersionDropdownOpen} onOpenChange={setIsVersionDropdownOpen}>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 px-2 text-[11px] gap-1 rounded-lg transition-colors duration-150"
                      style={{ color: "hsl(220 10% 50%)" }}
                      title="View version history"
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                        e.currentTarget.style.color = "hsl(220 10% 90%)";
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = "transparent";
                        e.currentTarget.style.color = "hsl(220 10% 50%)";
                      }}
                    >
                      <History className="w-3 h-3" />
                      v{selectedVersion}
                      <ChevronDown className="w-3 h-3" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent
                    align="end"
                    className="w-36"
                    style={{
                      background: "hsl(220 10% 14%)",
                      backdropFilter: "blur(20px)",
                      border: "1px solid hsla(220 10% 100% / 0.08)",
                      boxShadow: "0 8px 32px hsla(220 10% 0% / 0.4)",
                    }}
                  >
                    {Array.from({ length: plan.metadata.version }, (_, i) => plan.metadata.version - i).map((version) => {
                      const isSelected = version === selectedVersion;
                      const isLatest = version === plan.metadata.version;
                      return (
                        <DropdownMenuItem
                          key={version}
                          onClick={() => handleVersionSelect(version)}
                          className="text-[12px] cursor-pointer px-3 py-2"
                          style={{
                            background: isSelected ? "hsla(14 100% 60% / 0.1)" : "transparent",
                            borderLeft: isSelected ? "2px solid hsl(14 100% 60%)" : "2px solid transparent",
                          }}
                        >
                          <span className="flex items-center gap-2 w-full">
                            {isSelected && (
                              <span
                                className="w-1.5 h-1.5 rounded-full"
                                style={{ background: "hsl(14 100% 60%)" }}
                              />
                            )}
                            <span>v{version}</span>
                            {isLatest && (
                              <span className="ml-auto" style={{ color: "hsl(220 10% 50%)" }}>
                                (latest)
                              </span>
                            )}
                          </span>
                        </DropdownMenuItem>
                      );
                    })}
                  </DropdownMenuContent>
                </DropdownMenu>
              )}

              <Button
                variant="ghost"
                size="sm"
                onClick={onEdit}
                className="h-7 w-7 p-0 rounded-lg transition-colors duration-150"
                style={{ color: "hsl(220 10% 50%)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                  e.currentTarget.style.color = "hsl(220 10% 90%)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "hsl(220 10% 50%)";
                }}
              >
                <FileEdit className="w-3.5 h-3.5" />
              </Button>

              <Button
                variant="ghost"
                size="sm"
                onClick={handleExport}
                className="h-7 w-7 p-0 rounded-lg transition-colors duration-150"
                style={{ color: "hsl(220 10% 50%)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.06)";
                  e.currentTarget.style.color = "hsl(220 10% 90%)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                  e.currentTarget.style.color = "hsl(220 10% 50%)";
                }}
              >
                <Download className="w-3.5 h-3.5" />
              </Button>
            </div>
          </div>
        </div>

        {/* Content */}
        <CollapsibleContent>
          <div
            className="mt-3 pl-6 pr-2 pb-4"
            style={{
              marginLeft: "16px",
              borderLeft: "2px solid hsla(14 100% 60% / 0.15)",
            }}
          >
            {/* Version banner when viewing historical */}
            {isViewingHistorical && (
              <div
                className="flex items-center justify-between mb-4 px-3 py-2.5 rounded-lg"
                style={{
                  background: "hsla(45 93% 50% / 0.1)",
                  border: "1px solid hsla(45 93% 50% / 0.2)",
                }}
              >
                <span className="text-[12px] font-medium" style={{ color: "hsl(45 93% 55%)" }}>
                  Viewing version {selectedVersion} of {plan.metadata.version}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleBackToLatest}
                  className="h-6 px-2.5 text-[11px] font-medium gap-1.5 rounded-md transition-colors duration-150"
                  style={{
                    color: "hsl(45 93% 55%)",
                    background: "hsla(45 93% 50% / 0.1)",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "hsla(45 93% 50% / 0.2)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "hsla(45 93% 50% / 0.1)";
                  }}
                >
                  <ArrowLeft className="w-3 h-3" />
                  Back to latest
                </Button>
              </div>
            )}

            {/* Loading state */}
            {isLoadingVersion ? (
              <div className="flex items-center justify-center py-12">
                <div className="flex flex-col items-center gap-3">
                  <Loader2 className="w-6 h-6 animate-spin" style={{ color: "hsl(14 100% 60%)" }} />
                  <span className="text-[12px]" style={{ color: "hsl(220 10% 50%)" }}>
                    Loading version...
                  </span>
                </div>
              </div>
            ) : displayContent ? (
              <div className="text-[13px] leading-relaxed">
                <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                  {displayContent}
                </ReactMarkdown>
              </div>
            ) : (
              <p
                className="text-[13px] italic py-8 text-center"
                style={{ color: "hsl(220 10% 50%)" }}
              >
                No content available
              </p>
            )}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
