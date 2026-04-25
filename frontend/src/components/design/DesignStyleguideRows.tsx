import {
  Check,
  ChevronDown,
  ChevronRight,
  Loader2,
  MessageSquare,
  Sparkles,
  X,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import type { DesignReviewState, DesignStyleguideItem } from "./designSystems";
import { PreviewBlock } from "./DesignStyleguidePreviews";

export function StyleguideRow({
  designSystemId,
  item,
  reviewState,
  isExpanded,
  isFeedbackOpen,
  feedbackDraft,
  onToggle,
  onApprove,
  onOpenFeedback,
  onFeedbackDraftChange,
  onSubmitFeedback,
  onCancelFeedback,
  onGenerateArtifact,
  isGeneratingArtifact,
  onOpenFocused,
}: {
  designSystemId: string;
  item: DesignStyleguideItem;
  reviewState: DesignReviewState;
  isExpanded: boolean;
  isFeedbackOpen: boolean;
  feedbackDraft: string;
  onToggle: () => void;
  onApprove: () => void;
  onOpenFeedback: () => void;
  onFeedbackDraftChange: (value: string) => void;
  onSubmitFeedback: () => void;
  onCancelFeedback: () => void;
  onGenerateArtifact: () => void;
  isGeneratingArtifact: boolean;
  onOpenFocused: () => void;
}) {
  const artifactKind = item.group === "ui_kit" ? "screen" : "component";
  const statusLabel =
    item.confidence === "low"
      ? "source review"
      : reviewState === "approved"
        ? "approved"
        : reviewState.replace("_", " ");
  const feedbackLabel =
    reviewState === "approved"
      ? "Reopen feedback"
      : reviewState === "needs_work"
        ? "Add feedback"
        : "Needs work";
  const sourceLabel = item.sourceRefs.length === 1 ? "source" : "sources";
  const previewLabel = item.previewArtifactId ?? "preview pending";
  return (
    <div className="rounded-lg border overflow-hidden" style={{ borderColor: "var(--overlay-weak)" }}>
      <button
        type="button"
        className="w-full px-3 py-2 flex items-center gap-2 text-left"
        onClick={onToggle}
        data-testid={`design-styleguide-row-${item.itemId}`}
      >
        {isExpanded ? <ChevronDown className="w-4 h-4" /> : <ChevronRight className="w-4 h-4" />}
        <div className="min-w-0 flex-1">
          <div className="text-[13px] font-medium truncate" style={{ color: "var(--text-primary)" }}>
            {item.label}
          </div>
          <div className="text-[11px] truncate" style={{ color: "var(--text-muted)" }}>
            {item.summary}
          </div>
        </div>
        <span
          className="shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-medium"
          style={{
            borderColor: item.confidence === "low" ? "var(--status-warning-border)" : "var(--overlay-faint)",
            background: item.confidence === "low" ? "var(--status-warning-muted)" : "transparent",
            color: item.confidence === "low" ? "var(--status-warning)" : "var(--text-muted)",
          }}
        >
          {statusLabel}
        </span>
      </button>

      {isExpanded && (
        <div className="border-t p-3 space-y-3" style={{ borderColor: "var(--overlay-faint)" }}>
          <PreviewBlock designSystemId={designSystemId} item={item} />
          <div className="flex items-center gap-2">
            <Button
              type="button"
              size="sm"
              className="h-8 gap-2"
              onClick={onApprove}
              data-testid={`design-approve-${item.itemId}`}
            >
              <Check className="w-4 h-4" />
              {reviewState === "approved" ? "Approved" : "Looks good"}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 gap-2"
              onClick={onOpenFeedback}
              data-testid={`design-needs-work-${item.itemId}`}
            >
              <MessageSquare className="w-4 h-4" />
              {feedbackLabel}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 gap-2"
              disabled={isGeneratingArtifact || !item.isPersisted}
              onClick={onGenerateArtifact}
              data-testid={`design-generate-artifact-${item.itemId}`}
            >
              {isGeneratingArtifact ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <Sparkles className="w-4 h-4" />
              )}
              {artifactKind === "screen" ? "Generate screen" : "Generate component"}
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-8"
              onClick={onOpenFocused}
              data-testid={`design-open-full-preview-${item.itemId}`}
            >
              Open full preview
            </Button>
          </div>
          {isFeedbackOpen && (
            <div className="space-y-2" data-testid="design-feedback-composer">
              <div
                className="rounded-lg border px-3 py-2 text-[11px] leading-5"
                style={{
                  borderColor: "var(--overlay-faint)",
                  background: "var(--bg-surface)",
                  color: "var(--text-muted)",
                }}
                data-testid="design-feedback-context"
              >
                <div>Item: {item.group.replace("_", " ")} / {item.label}</div>
                <div>Preview: {previewLabel}</div>
                <div>
                  Source refs: {item.sourceRefs.length} {sourceLabel}
                  {item.sourceRefs[0] ? ` - ${item.sourceRefs[0].path}` : ""}
                </div>
              </div>
              <textarea
                value={feedbackDraft}
                onChange={(event) => onFeedbackDraftChange(event.target.value)}
                className="w-full min-h-20 rounded-lg border bg-transparent p-2 text-[12px] outline-none"
                style={{ borderColor: "var(--overlay-weak)", color: "var(--text-primary)" }}
                placeholder="Feedback"
              />
              <div className="flex justify-end gap-2">
                <Button type="button" variant="ghost" size="sm" onClick={onCancelFeedback}>
                  Cancel
                </Button>
                <Button type="button" size="sm" onClick={onSubmitFeedback}>
                  Send feedback to Design
                </Button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function FocusedItemDrawer({
  designSystemId,
  item,
  reviewState,
  isGeneratingArtifact,
  onClose,
  onApprove,
  onOpenFeedback,
  onGenerateArtifact,
}: {
  designSystemId: string;
  item: DesignStyleguideItem;
  reviewState: DesignReviewState;
  isGeneratingArtifact: boolean;
  onClose: () => void;
  onApprove: () => void;
  onOpenFeedback: () => void;
  onGenerateArtifact: () => void;
}) {
  const artifactKind = item.group === "ui_kit" ? "screen" : "component";

  return (
    <div
      className="fixed inset-y-0 right-0 z-50 w-[min(520px,100vw)] border-l shadow-xl flex flex-col"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-base)" }}
      data-testid="design-focused-item-drawer"
    >
      <header className="h-14 px-4 border-b flex items-center gap-3 shrink-0" style={{ borderColor: "var(--overlay-faint)" }}>
        <div className="min-w-0 flex-1">
          <div className="text-[13px] font-semibold truncate" style={{ color: "var(--text-primary)" }}>
            {item.label}
          </div>
          <div className="text-[11px]" style={{ color: "var(--text-muted)" }}>
            {reviewState.replace("_", " ")} / {item.sourceRefs.length} sources
          </div>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0"
          aria-label="Close preview"
          onClick={onClose}
          data-testid="design-close-focused-preview"
        >
          <X className="w-4 h-4" />
        </Button>
      </header>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <PreviewBlock designSystemId={designSystemId} item={item} />

        <section className="space-y-2">
          <h3 className="text-[12px] font-semibold" style={{ color: "var(--text-muted)" }}>
            Sources
          </h3>
          <div className="space-y-1">
            {item.sourceRefs.length ? (
              item.sourceRefs.map((sourceRef) => (
                <div
                  key={`${sourceRef.projectId}:${sourceRef.path}:${sourceRef.line ?? ""}`}
                  className="rounded-md border px-2.5 py-2 text-[12px]"
                  style={{ borderColor: "var(--overlay-faint)", color: "var(--text-secondary)" }}
                >
                  <div className="truncate" style={{ color: "var(--text-primary)" }}>
                    {sourceRef.path}
                  </div>
                  <div className="text-[11px]" style={{ color: "var(--text-muted)" }}>
                    {sourceRef.projectId}{sourceRef.line ? `:${sourceRef.line}` : ""}
                  </div>
                </div>
              ))
            ) : (
              <div className="text-[12px]" style={{ color: "var(--text-muted)" }}>
                No source references stored for this row.
              </div>
            )}
          </div>
        </section>

        <section className="space-y-2">
          <h3 className="text-[12px] font-semibold" style={{ color: "var(--text-muted)" }}>
            Activity
          </h3>
          <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
            Latest row update is stored in the current styleguide version. Feedback and generated artifacts stay attached to this design conversation.
          </div>
        </section>
      </div>

      <footer className="border-t p-3 flex flex-wrap justify-end gap-2 shrink-0" style={{ borderColor: "var(--overlay-faint)" }}>
        <Button type="button" variant="outline" size="sm" onClick={onOpenFeedback}>
          Needs work
        </Button>
        <Button type="button" variant="outline" size="sm" disabled={isGeneratingArtifact || !item.isPersisted} onClick={onGenerateArtifact}>
          {artifactKind === "screen" ? "Generate screen" : "Generate component"}
        </Button>
        <Button type="button" size="sm" onClick={onApprove}>
          Looks good
        </Button>
      </footer>
    </div>
  );
}
