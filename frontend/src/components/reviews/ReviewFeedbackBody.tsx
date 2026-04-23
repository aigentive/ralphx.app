import { useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { markdownComponents } from "@/components/Chat/MessageItem.markdown";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import {
  buildReviewFeedbackPreview,
  sanitizeReviewFeedbackText,
} from "@/lib/review-feedback";

interface ReviewFeedbackBodyProps {
  summary?: string | null;
  notes?: string | null;
  mode?: "markdown" | "plain";
  previewCharLimit?: number;
  dialogTitle?: string;
  dialogDescription?: string;
  fullButtonLabel?: string;
  previewClassName?: string;
  dialogBodyClassName?: string;
}

function MarkdownBody({ content, className }: { content: string; className?: string }) {
  return (
    <div className={cn("prose prose-sm prose-invert max-w-none", className)}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
        {content}
      </ReactMarkdown>
    </div>
  );
}

export function ReviewFeedbackBody({
  summary,
  notes,
  mode = "markdown",
  previewCharLimit = 900,
  dialogTitle = "Full feedback",
  dialogDescription = "Full feedback in a scrollable view.",
  fullButtonLabel = "View full details",
  previewClassName,
  dialogBodyClassName,
}: ReviewFeedbackBodyProps) {
  const [isDialogOpen, setIsDialogOpen] = useState(false);

  const sanitizedSummary = useMemo(
    () => (summary ? sanitizeReviewFeedbackText(summary) : null),
    [summary]
  );
  const sanitizedNotes = useMemo(
    () => (notes ? sanitizeReviewFeedbackText(notes) : null),
    [notes]
  );

  const previewText = useMemo(() => {
    if (!sanitizedNotes) {
      return null;
    }
    return buildReviewFeedbackPreview(sanitizedNotes, previewCharLimit);
  }, [previewCharLimit, sanitizedNotes]);
  const isNotesTruncated = useMemo(() => {
    if (!sanitizedNotes) return false;
    return sanitizedNotes.replace(/\s+/g, " ").trim().length > previewCharLimit;
  }, [previewCharLimit, sanitizedNotes]);

  const showSummary = !!sanitizedSummary;
  const showFullBody = !!sanitizedNotes;
  const renderFullNotes = showFullBody && isNotesTruncated;
  const notesIncludeSummary =
    showSummary &&
    showFullBody &&
    sanitizedNotes
      .replace(/\s+/g, " ")
      .trim()
      .startsWith(sanitizedSummary.replace(/\s+/g, " ").trim());

  const previewBody = showFullBody && !isNotesTruncated
    ? showSummary && !notesIncludeSummary
      ? `${sanitizedSummary}\n\n${sanitizedNotes}`
      : sanitizedNotes
    : showSummary
    ? sanitizedSummary
    : showFullBody
    ? previewText
    : null;

  if (!previewBody) {
    return null;
  }

  return (
    <>
      <div className={previewClassName}>
        {mode === "markdown" && (showSummary || !isNotesTruncated) ? (
          <MarkdownBody content={previewBody} />
        ) : mode === "plain" ? (
          <pre className="whitespace-pre-wrap break-words font-inherit">
            {previewBody}
          </pre>
        ) : (
          <div className="whitespace-pre-wrap break-words">{previewBody}</div>
        )}
      </div>

      {renderFullNotes && (
        <>
          <button
            type="button"
            className="mt-2 text-[12px] font-medium text-[var(--accent-primary)] hover:text-[var(--accent-primary-hover)]"
            onClick={() => setIsDialogOpen(true)}
          >
            {fullButtonLabel}
          </button>
          <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
            <DialogContent className="sm:max-w-3xl max-h-[80vh] overflow-hidden">
              <DialogHeader>
                <DialogTitle>{dialogTitle}</DialogTitle>
                <DialogDescription>{dialogDescription}</DialogDescription>
              </DialogHeader>
              <div className="px-6 pb-6">
                <div
                  className={cn(
                    "max-h-[56vh] overflow-y-auto rounded-lg bg-[var(--overlay-faint)] p-4",
                    dialogBodyClassName
                  )}
                >
                  {mode === "markdown" ? (
                    <MarkdownBody content={sanitizedNotes} />
                  ) : (
                    <pre className="whitespace-pre-wrap break-words font-mono text-[12px] text-text-primary/80">
                      {sanitizedNotes}
                    </pre>
                  )}
                </div>
              </div>
            </DialogContent>
          </Dialog>
        </>
      )}
    </>
  );
}
