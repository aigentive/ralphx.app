/**
 * PlanDisplay - Display plan artifacts in IdeationView
 *
 * Features:
 * - Shows plan artifact title and markdown content
 * - Collapsible for space management
 * - Edit and Export buttons in header
 * - "Approve Plan" button when require_plan_approval is true
 * - Plan-proposal linkage indicator
 */

import { useState, useCallback } from "react";
import { FileEdit, Download, CheckCircle2, ChevronDown, ChevronRight, FileText } from "lucide-react";
import ReactMarkdown from "react-markdown";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import type { Artifact } from "@/types/artifact";

// ============================================================================
// Types
// ============================================================================

export interface PlanDisplayProps {
  /** The plan artifact to display */
  plan: Artifact;
  /** Whether to show the approve button (when require_plan_approval is true) */
  showApprove?: boolean;
  /** Number of proposals linked to this plan */
  linkedProposalsCount?: number;
  /** Callback when edit button is clicked */
  onEdit?: () => void;
  /** Callback when export button is clicked */
  onExport?: () => void;
  /** Callback when approve button is clicked */
  onApprove?: () => void;
  /** Whether the plan is approved (disables approve button) */
  isApproved?: boolean;
}

// ============================================================================
// Markdown Components
// ============================================================================

const markdownComponents = {
  a: ({ href, children, ...props }: React.AnchorHTMLAttributes<HTMLAnchorElement>) => (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="underline hover:no-underline text-[var(--accent-primary)]"
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
          className={`block p-3 rounded text-sm overflow-x-auto bg-[var(--bg-elevated)] ${className || ""}`}
          {...props}
        >
          {children}
        </code>
      );
    }
    return (
      <code className="px-1 py-0.5 rounded text-sm bg-[var(--bg-elevated)]" {...props}>
        {children}
      </code>
    );
  },
  pre: ({ children, ...props }: React.HTMLAttributes<HTMLPreElement>) => (
    <pre className="my-2 rounded overflow-hidden bg-[var(--bg-elevated)]" {...props}>
      {children}
    </pre>
  ),
  p: ({ children, ...props }: React.HTMLAttributes<HTMLParagraphElement>) => (
    <p className="mb-2 last:mb-0" {...props}>
      {children}
    </p>
  ),
  ul: ({ children, ...props }: React.HTMLAttributes<HTMLUListElement>) => (
    <ul className="list-disc list-inside mb-2 space-y-1" {...props}>
      {children}
    </ul>
  ),
  ol: ({ children, ...props }: React.HTMLAttributes<HTMLOListElement>) => (
    <ol className="list-decimal list-inside mb-2 space-y-1" {...props}>
      {children}
    </ol>
  ),
  li: ({ children, ...props }: React.LiHTMLAttributes<HTMLLIElement>) => (
    <li className="mb-1" {...props}>
      {children}
    </li>
  ),
  h1: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h1 className="text-2xl font-semibold mb-3 mt-4 first:mt-0" {...props}>
      {children}
    </h1>
  ),
  h2: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h2 className="text-xl font-semibold mb-2 mt-3" {...props}>
      {children}
    </h2>
  ),
  h3: ({ children, ...props }: React.HTMLAttributes<HTMLHeadingElement>) => (
    <h3 className="text-lg font-semibold mb-2 mt-3" {...props}>
      {children}
    </h3>
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

  // Get plan content text
  const planContent = plan.content.type === "inline" ? plan.content.text : "";

  // Handle export - download as markdown file
  const handleExport = useCallback(() => {
    if (onExport) {
      onExport();
      return;
    }

    // Default export behavior
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
    <Card className="overflow-hidden border-[var(--border-primary)] bg-[var(--bg-elevated)]">
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-[var(--border-primary)]">
          <div className="flex items-center gap-3 flex-1 min-w-0">
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 w-6 p-0 hover:bg-[var(--bg-base)]"
              >
                {isOpen ? (
                  <ChevronDown className="h-4 w-4" />
                ) : (
                  <ChevronRight className="h-4 w-4" />
                )}
              </Button>
            </CollapsibleTrigger>

            <FileText className="h-5 w-5 text-[var(--accent-primary)] flex-shrink-0" />

            <div className="flex-1 min-w-0">
              <h3 className="font-semibold text-base text-[var(--text-primary)] truncate">
                {plan.name}
              </h3>
              {linkedProposalsCount > 0 && (
                <p className="text-sm text-[var(--text-tertiary)]">
                  {linkedProposalsCount} {linkedProposalsCount === 1 ? "proposal" : "proposals"}{" "}
                  linked
                </p>
              )}
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-2">
            {showApprove && !isApproved && (
              <Button
                variant="default"
                size="sm"
                onClick={onApprove}
                className="bg-[var(--accent-primary)] hover:bg-[var(--accent-hover)] text-white"
              >
                <CheckCircle2 className="h-4 w-4 mr-1.5" />
                Approve Plan
              </Button>
            )}

            {isApproved && (
              <Badge className="bg-green-500/10 text-green-500 border-green-500/20">
                <CheckCircle2 className="h-3.5 w-3.5 mr-1" />
                Approved
              </Badge>
            )}

            <Button
              variant="ghost"
              size="sm"
              onClick={onEdit}
              className="hover:bg-[var(--bg-base)]"
            >
              <FileEdit className="h-4 w-4 mr-1.5" />
              Edit
            </Button>

            <Button
              variant="ghost"
              size="sm"
              onClick={handleExport}
              className="hover:bg-[var(--bg-base)]"
            >
              <Download className="h-4 w-4 mr-1.5" />
              Export
            </Button>
          </div>
        </div>

        {/* Content */}
        <CollapsibleContent>
          <div className="p-4">
            {planContent ? (
              <div className="prose prose-sm max-w-none text-[var(--text-primary)]">
                <ReactMarkdown components={markdownComponents}>{planContent}</ReactMarkdown>
              </div>
            ) : (
              <p className="text-[var(--text-tertiary)] italic">No content</p>
            )}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </Card>
  );
}
