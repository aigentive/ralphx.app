/**
 * PlanDisplay - Display plan artifacts in IdeationView
 *
 * Refined design matching app aesthetic:
 * - Subtle, minimal chrome
 * - Smooth transitions
 * - Warm accent integration
 */

import { useState, useCallback } from "react";
import { FileEdit, Download, CheckCircle2, ChevronDown, FileText, Sparkles } from "lucide-react";
import ReactMarkdown from "react-markdown";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
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
}: PlanDisplayProps) {
  const [isOpen, setIsOpen] = useState(true);

  const planContent = plan.content.type === "inline" ? plan.content.text : "";

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
            {planContent ? (
              <div className="text-sm">
                <ReactMarkdown components={markdownComponents}>
                  {planContent}
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
