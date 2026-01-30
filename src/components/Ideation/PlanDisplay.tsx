/**
 * PlanDisplay - Display plan artifacts in IdeationView
 *
 * Refined design matching app aesthetic:
 * - Subtle, minimal chrome
 * - Smooth transitions
 * - Warm accent integration
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
  // Use controlled state if isExpanded prop is provided, otherwise use internal state
  // Default to collapsed (false) for initial render
  const [internalIsOpen, setInternalIsOpen] = useState(false);
  const isOpen = isExpanded !== undefined ? isExpanded : internalIsOpen;
  const setIsOpen = onExpandedChange ?? setInternalIsOpen;

  // Version selector state
  const [selectedVersion, setSelectedVersion] = useState(plan.metadata.version);
  const [historicalContent, setHistoricalContent] = useState<string | null>(null);
  const [isLoadingVersion, setIsLoadingVersion] = useState(false);

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
    <div className="group">
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        {/* Minimal Header */}
        <div className="flex items-center gap-2 mb-2">
          <CollapsibleTrigger asChild>
            <button
              className={cn(
                "flex items-center gap-2 text-left flex-1 min-w-0",
                "hover:opacity-80 transition-opacity"
              )}
            >
              <div className="flex items-center justify-center w-5 h-5 rounded bg-accent-primary/10">
                <FileText className="w-3 h-3 text-accent-primary" />
              </div>

              <span className="text-sm font-medium text-text-primary truncate">
                {plan.name}
              </span>

              <span className="text-xs text-text-muted px-1.5 py-0.5 rounded bg-white/[0.04] border border-white/[0.06]">
                v{plan.metadata.version}
              </span>

              {linkedProposalsCount > 0 && (
                <span className="text-xs text-text-muted">
                  · {linkedProposalsCount} linked
                </span>
              )}

              <ChevronDown
                className={cn(
                  "w-3.5 h-3.5 text-text-muted transition-transform duration-200",
                  !isOpen && "-rotate-90"
                )}
              />
            </button>
          </CollapsibleTrigger>

          {/* Actions - appear on hover */}
          <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            {showApprove && !isApproved && (
              <Button
                variant="ghost"
                size="sm"
                onClick={onApprove}
                className="h-6 px-2 text-xs gap-1 text-accent-primary hover:text-accent-primary hover:bg-accent-primary/10"
              >
                <Sparkles className="w-3 h-3" />
                Approve
              </Button>
            )}

            {isApproved && (
              <span className="flex items-center gap-1 text-xs text-green-500 px-2">
                <CheckCircle2 className="w-3 h-3" />
                Approved
              </span>
            )}

            {plan.metadata.version > 1 && (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 px-2 text-xs gap-1 text-text-muted hover:text-text-primary"
                    title="View version history"
                  >
                    <History className="w-3 h-3" />
                    v{selectedVersion}
                    <ChevronDown className="w-3 h-3" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-36 bg-[#1a1a1a] border-white/[0.1]">
                  {Array.from({ length: plan.metadata.version }, (_, i) => plan.metadata.version - i).map((version) => {
                    const isSelected = version === selectedVersion;
                    const isLatest = version === plan.metadata.version;
                    return (
                      <DropdownMenuItem
                        key={version}
                        onClick={() => setSelectedVersion(version)}
                        className={cn(
                          "text-xs cursor-pointer px-3 py-2",
                          isSelected && "bg-[var(--accent-muted)] border-l-2 border-[var(--accent-primary)]"
                        )}
                      >
                        <span className="flex items-center gap-2">
                          {isSelected && <span className="w-1.5 h-1.5 rounded-full bg-[var(--accent-primary)]" />}
                          <span>v{version}</span>
                          {isLatest && <span className="text-text-muted ml-auto">(latest)</span>}
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
              className="h-6 w-6 p-0 text-text-muted hover:text-text-primary"
            >
              <FileEdit className="w-3.5 h-3.5" />
            </Button>

            <Button
              variant="ghost"
              size="sm"
              onClick={handleExport}
              className="h-6 w-6 p-0 text-text-muted hover:text-text-primary"
            >
              <Download className="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        {/* Content */}
        <CollapsibleContent>
          <div
            className={cn(
              "pl-7 pr-2 pb-4",
              "border-l border-white/[0.04] ml-2.5"
            )}
          >
            {/* Version banner when viewing historical */}
            {isViewingHistorical && (
              <div className="flex items-center justify-between mb-3 px-3 py-2 rounded-md bg-amber-500/10 border border-amber-500/20">
                <span className="text-xs text-amber-400">
                  Viewing version {selectedVersion} of {plan.metadata.version}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleBackToLatest}
                  className="h-6 px-2 text-xs gap-1 text-amber-400 hover:text-amber-300 hover:bg-amber-500/10"
                >
                  <ArrowLeft className="w-3 h-3" />
                  Back to latest
                </Button>
              </div>
            )}

            {/* Loading state */}
            {isLoadingVersion ? (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="w-5 h-5 text-text-muted animate-spin" />
              </div>
            ) : displayContent ? (
              <div className="text-sm">
                <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                  {displayContent}
                </ReactMarkdown>
              </div>
            ) : (
              <p className="text-sm text-text-muted italic">No content</p>
            )}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
