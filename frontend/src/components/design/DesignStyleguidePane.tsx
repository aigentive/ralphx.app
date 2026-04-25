import {
  Check,
  ChevronDown,
  ChevronRight,
  Download,
  Loader2,
  MessageSquare,
  Package,
  Sparkles,
  X,
} from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import type {
  DesignStyleguidePreviewResponse,
  ExportDesignSystemPackageResponse,
} from "@/api/design";
import type { DesignReviewState, DesignStyleguideItem, DesignSystem } from "./designSystems";
import {
  useApproveDesignStyleguideItem,
  useCreateDesignStyleguideFeedback,
  useGenerateDesignArtifact,
  useDesignStyleguidePreview,
} from "./useProjectDesignSystems";

type DesignPreviewContent = DesignStyleguidePreviewResponse["content"];

const FILTERS: Array<{ id: "all" | DesignReviewState; label: string }> = [
  { id: "all", label: "All" },
  { id: "needs_review", label: "Needs review" },
  { id: "needs_work", label: "Needs work" },
  { id: "approved", label: "Approved" },
  { id: "stale", label: "Stale" },
];

interface DesignStyleguidePaneProps {
  designSystem: DesignSystem | null;
  isGeneratingStyleguide?: boolean;
  isExportingPackage?: boolean;
  exportPackage?: ExportDesignSystemPackageResponse | null;
  generationResult?: {
    itemCount: number;
    caveatCount: number;
    schemaVersionId: string | null;
  } | null;
  onGenerateStyleguide?: () => void;
  onExportPackage?: () => void;
}

export function DesignStyleguidePane({
  designSystem,
  isGeneratingStyleguide = false,
  isExportingPackage = false,
  exportPackage = null,
  generationResult = null,
  onGenerateStyleguide,
  onExportPackage,
}: DesignStyleguidePaneProps) {
  const [activeFilter, setActiveFilter] = useState<"all" | DesignReviewState>("all");
  const [expandedItemId, setExpandedItemId] = useState<string | null>(null);
  const [feedbackItemId, setFeedbackItemId] = useState<string | null>(null);
  const [focusedItemId, setFocusedItemId] = useState<string | null>(null);
  const [approvalOverrides, setApprovalOverrides] = useState<Record<string, DesignReviewState>>({});
  const [feedbackDraft, setFeedbackDraft] = useState("");
  const [generatedArtifact, setGeneratedArtifact] = useState<{
    artifactId: string;
    kind: "screen" | "component";
    name: string;
  } | null>(null);
  const approveMutation = useApproveDesignStyleguideItem();
  const feedbackMutation = useCreateDesignStyleguideFeedback();
  const generateArtifactMutation = useGenerateDesignArtifact();
  const hasPersistedItems =
    designSystem?.groups.some((group) => group.items.some((item) => item.isPersisted)) ?? false;
  const canGenerateStyleguide =
    !!designSystem && designSystem.status !== "archived" && designSystem.sourceCount > 0;
  const generateLabel = hasPersistedItems || designSystem?.status === "ready" ? "Regenerate" : "Generate";
  const generationRowLabel = generationResult?.itemCount === 1 ? "row" : "rows";
  const generationCaveatLabel = generationResult?.caveatCount === 1 ? "caveat" : "caveats";
  let generationStatusText =
    "Styleguide generation requested. Design will publish the first review rows from chat.";
  if (generationResult && generationResult.itemCount > 0) {
    generationStatusText = `Styleguide refresh requested with ${generationResult.itemCount} existing review ${generationRowLabel}`;
    if (generationResult.caveatCount > 0) {
      generationStatusText = `${generationStatusText} and ${generationResult.caveatCount} ${generationCaveatLabel}`;
    }
    generationStatusText = `${generationStatusText}.`;
  }
  const hasStyleguideRows =
    designSystem?.groups.some((group) => group.items.length > 0) ?? false;

  const reviewCounts = useMemo(() => {
    const counts: Record<DesignReviewState, number> = {
      needs_review: 0,
      approved: 0,
      needs_work: 0,
      stale: 0,
    };
    if (!designSystem) {
      return counts;
    }
    for (const group of designSystem.groups) {
      for (const item of group.items) {
        counts[approvalOverrides[item.id] ?? item.reviewState] += 1;
      }
    }
    return counts;
  }, [approvalOverrides, designSystem]);

  const focusedItem = useMemo(() => {
    if (!designSystem || !focusedItemId) {
      return null;
    }
    return designSystem.groups
      .flatMap((group) => group.items)
      .find((item) => item.id === focusedItemId) ?? null;
  }, [designSystem, focusedItemId]);

  if (!designSystem) {
    return (
      <aside className="h-full border-l flex items-center justify-center px-6" style={{ borderColor: "var(--overlay-faint)" }}>
        <div className="text-sm" style={{ color: "var(--text-muted)" }}>
          Select a design system
        </div>
      </aside>
    );
  }

  const approveItem = (item: DesignStyleguideItem) => {
    if (item.isPersisted) {
      approveMutation.mutate(
        { designSystemId: designSystem.id, itemId: item.itemId },
        {
          onSuccess: (updatedItem) => {
            setApprovalOverrides((current) => ({ ...current, [updatedItem.id]: "approved" }));
            setFeedbackItemId(null);
          },
        },
      );
      return;
    }
    setApprovalOverrides((current) => ({ ...current, [item.id]: "approved" }));
    setFeedbackItemId(null);
  };

  const submitFeedback = (item: DesignStyleguideItem) => {
    const feedback = feedbackDraft.trim();
    if (!feedback) {
      return;
    }
    if (item.isPersisted) {
      feedbackMutation.mutate(
        {
          designSystemId: designSystem.id,
          itemId: item.itemId,
          feedback,
          conversationId: designSystem.conversationId ?? undefined,
        },
        {
          onSuccess: (response) => {
            setApprovalOverrides((current) => ({
              ...current,
              [response.item.id]: "needs_work",
            }));
            setFeedbackDraft("");
            setFeedbackItemId(null);
            setExpandedItemId(response.item.id);
          },
        },
      );
      return;
    }
    setApprovalOverrides((current) => ({ ...current, [item.id]: "needs_work" }));
    setFeedbackDraft("");
    setFeedbackItemId(null);
    setExpandedItemId(item.id);
  };

  const generateArtifactFromItem = (item: DesignStyleguideItem) => {
    const kind = item.group === "ui_kit" ? "screen" : "component";
    generateArtifactMutation.mutate(
      {
        designSystemId: designSystem.id,
        artifactKind: kind,
        name: `${item.label} ${kind}`,
        brief: item.summary,
        sourceItemId: item.itemId,
      },
      {
        onSuccess: (response) => {
          setGeneratedArtifact({
            artifactId: response.artifactId,
            kind: response.artifactKind,
            name: response.name,
          });
        },
      },
    );
  };

  return (
    <aside
      className="h-full border-l flex flex-col min-w-0"
      style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
      data-testid="design-styleguide-pane"
    >
      <header className="h-12 px-4 border-b flex items-center gap-3 shrink-0" style={{ borderColor: "var(--overlay-faint)" }}>
        <div className="min-w-0 flex-1">
          <div className="text-[13px] font-semibold truncate" style={{ color: "var(--text-primary)" }}>
            Styleguide
          </div>
          <div className="text-[11px]" style={{ color: "var(--text-muted)" }}>
            v{designSystem.version}
          </div>
        </div>
        {canGenerateStyleguide && (
          <Button
            type="button"
            size="sm"
            className="gap-2"
            disabled={isGeneratingStyleguide || !onGenerateStyleguide}
            onClick={onGenerateStyleguide}
            data-testid="design-generate-styleguide"
          >
            {isGeneratingStyleguide ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Sparkles className="w-4 h-4" />
            )}
            {isGeneratingStyleguide ? "Generating" : generateLabel}
          </Button>
        )}
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="gap-2"
          disabled={designSystem.version === "draft" || isExportingPackage || !onExportPackage}
          onClick={onExportPackage}
          data-testid="design-export-package"
        >
          {isExportingPackage ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <Download className="w-4 h-4" />
          )}
          {isExportingPackage ? "Exporting" : "Export"}
        </Button>
      </header>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <section className="space-y-2">
          <div className="text-[13px] leading-5" style={{ color: "var(--text-secondary)" }}>
            {designSystem.readySummary}
          </div>
          {isGeneratingStyleguide && (
            <div
              className="rounded-lg border px-3 py-2 text-[12px] space-y-2"
              style={{
                borderColor: "var(--accent-border)",
                background: "var(--accent-muted)",
                color: "var(--text-secondary)",
              }}
              data-testid="design-generating-state"
            >
              <div>Analyzing selected sources and publishing the styleguide.</div>
              <div
                className="h-1.5 overflow-hidden rounded-full"
                style={{ background: "var(--overlay-faint)" }}
              >
                <div
                  className="h-full w-1/3 rounded-full"
                  style={{ background: "var(--accent-primary)" }}
                />
              </div>
              <div style={{ color: "var(--text-muted)" }}>
                Chat remains available while the styleguide is generated.
              </div>
            </div>
          )}
          {generationResult && !isGeneratingStyleguide && (
            <div
              className="rounded-lg border px-3 py-2 text-[12px]"
              style={{
                borderColor: generationResult.caveatCount > 0 ? "var(--status-warning-border)" : "var(--accent-border)",
                background: generationResult.caveatCount > 0 ? "var(--status-warning-muted)" : "var(--accent-muted)",
                color: "var(--text-secondary)",
              }}
              data-testid="design-generation-result"
            >
              {generationStatusText}
            </div>
          )}
          {exportPackage && (
            <div
              className="rounded-lg border p-3 text-[12px]"
              style={{
                borderColor: "var(--overlay-weak)",
                background: "var(--bg-surface)",
                color: "var(--text-secondary)",
              }}
              data-testid="design-export-result"
            >
              <div className="flex items-start gap-3">
                <Package className="mt-0.5 h-4 w-4 shrink-0" style={{ color: "var(--accent-primary)" }} />
                <div className="min-w-0 flex-1 space-y-1">
                  <div className="font-semibold" style={{ color: "var(--text-primary)" }}>
                    Export ready
                  </div>
                  <div className="leading-5">
                    Package artifact{" "}
                    <code className="rounded px-1 py-0.5 text-[11px]" style={{ background: "var(--overlay-faint)" }}>
                      {exportPackage.artifactId}
                    </code>{" "}
                    {exportPackage.filePath ? "was saved by RalphX." : "is stored in RalphX."}
                  </div>
                  {exportPackage.filePath && (
                    <div className="truncate" style={{ color: "var(--text-muted)" }}>
                      {exportPackage.filePath}
                    </div>
                  )}
                  <div style={{ color: "var(--text-muted)" }}>
                    Schema {exportPackage.schemaVersionId.slice(0, 8)} /{" "}
                    {exportPackage.redacted ? "absolute paths redacted" : "full provenance included"}
                  </div>
                </div>
              </div>
            </div>
          )}
          {generatedArtifact && (
            <div
              className="rounded-lg border px-3 py-2 text-[12px]"
              style={{
                borderColor: "var(--overlay-weak)",
                background: "var(--bg-surface)",
                color: "var(--text-secondary)",
              }}
              data-testid="design-generated-artifact-result"
            >
              Generated {generatedArtifact.kind} artifact {generatedArtifact.artifactId.slice(0, 8)} from {generatedArtifact.name}.
            </div>
          )}
          <div className="flex flex-wrap gap-1.5" data-testid="design-review-filters">
            {FILTERS.map((filter) => (
              <button
                key={filter.id}
                type="button"
                onClick={() => setActiveFilter(filter.id)}
                className="h-7 rounded-full border px-2.5 text-[11px] font-medium"
                style={{
                  borderColor: activeFilter === filter.id ? "var(--accent-border)" : "var(--overlay-weak)",
                  background: activeFilter === filter.id ? "var(--accent-muted)" : "transparent",
                  color: activeFilter === filter.id ? "var(--accent-primary)" : "var(--text-secondary)",
                }}
              >
                {filter.label}
                {filter.id !== "all" ? ` ${reviewCounts[filter.id]}` : ""}
              </button>
            ))}
          </div>
        </section>

        {designSystem.caveats.map((caveat) => (
          <section
            key={caveat.id}
            className="rounded-lg border p-3"
            style={{
              borderColor: "var(--status-warning-border)",
              background: "var(--status-warning-muted)",
            }}
            data-testid="design-caveat"
          >
            <div className="text-[12px] font-semibold" style={{ color: "var(--text-primary)" }}>
              {caveat.title}
            </div>
            <div className="mt-1 text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
              {caveat.body}
            </div>
            {caveat.actionLabel && (
              <Button type="button" variant="outline" size="sm" className="mt-3 h-7">
                {caveat.actionLabel}
              </Button>
            )}
          </section>
        ))}

        {!hasStyleguideRows && !isGeneratingStyleguide && (
          <section
            className="rounded-lg border p-4 text-[12px]"
            style={{
              borderColor: "var(--overlay-weak)",
              background: "var(--bg-surface)",
              color: "var(--text-secondary)",
            }}
            data-testid="design-styleguide-empty"
          >
            <div className="font-semibold" style={{ color: "var(--text-primary)" }}>
              No styleguide rows yet
            </div>
            <div className="mt-1 leading-5">
              Generate a styleguide to analyze the selected source project and publish source-backed review rows.
            </div>
          </section>
        )}

        {designSystem.groups.map((group) => {
          const visibleItems = group.items.filter((item) => {
            const state = approvalOverrides[item.id] ?? item.reviewState;
            return activeFilter === "all" || state === activeFilter;
          });
          if (visibleItems.length === 0) {
            return null;
          }

          return (
            <section key={group.id} className="space-y-1.5" data-testid={`design-styleguide-group-${group.id}`}>
              <h3 className="text-[12px] font-semibold" style={{ color: "var(--text-muted)" }}>
                {group.label}
              </h3>
              <div className="space-y-1.5">
                {visibleItems.map((item) => (
                  <StyleguideRow
                    key={item.id}
                    designSystemId={designSystem.id}
                    item={item}
                    reviewState={approvalOverrides[item.id] ?? item.reviewState}
                    isExpanded={expandedItemId === item.id}
                    isFeedbackOpen={feedbackItemId === item.id}
                    feedbackDraft={feedbackDraft}
                    onToggle={() => setExpandedItemId(expandedItemId === item.id ? null : item.id)}
                    onApprove={() => approveItem(item)}
                    onOpenFeedback={() => {
                      setExpandedItemId(item.id);
                      setFeedbackItemId(item.id);
                    }}
                    onFeedbackDraftChange={setFeedbackDraft}
                    onSubmitFeedback={() => submitFeedback(item)}
                    onCancelFeedback={() => {
                      setFeedbackItemId(null);
                      setFeedbackDraft("");
                    }}
                    onGenerateArtifact={() => generateArtifactFromItem(item)}
                    isGeneratingArtifact={generateArtifactMutation.isPending}
                    onOpenFocused={() => setFocusedItemId(item.id)}
                  />
                ))}
              </div>
            </section>
          );
        })}
      </div>
      {focusedItem && (
        <FocusedItemDrawer
          designSystemId={designSystem.id}
          item={focusedItem}
          reviewState={approvalOverrides[focusedItem.id] ?? focusedItem.reviewState}
          isGeneratingArtifact={generateArtifactMutation.isPending}
          onClose={() => setFocusedItemId(null)}
          onApprove={() => approveItem(focusedItem)}
          onOpenFeedback={() => {
            setExpandedItemId(focusedItem.id);
            setFeedbackItemId(focusedItem.id);
            setFocusedItemId(null);
          }}
          onGenerateArtifact={() => generateArtifactFromItem(focusedItem)}
        />
      )}
    </aside>
  );
}

function StyleguideRow({
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

function FocusedItemDrawer({
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

function PreviewBlock({ designSystemId, item }: { designSystemId: string; item: DesignStyleguideItem }) {
  const persistedPreviewArtifactId = item.isPersisted ? item.previewArtifactId ?? null : null;
  const previewQuery = useDesignStyleguidePreview(
    designSystemId,
    persistedPreviewArtifactId,
  );
  const preview = previewQuery.data?.content;
  const sourceCount = preview?.source_refs.length ?? item.sourceRefs.length;
  const previewKind = preview?.preview_kind ?? previewKindForItem(item);

  if (!item.isPersisted) {
    return (
      <PreviewSample
        item={item}
        preview={undefined}
        previewKind={previewKind}
        sourceCount={sourceCount}
      />
    );
  }

  if (!item.previewArtifactId) {
    return (
      <div
        className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
        style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
        data-testid="design-preview-empty"
      >
        <Package className="w-4 h-4" style={{ color: "var(--text-muted)" }} />
        <span className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
          Preview pending
        </span>
      </div>
    );
  }

  if (previewQuery.isLoading) {
    return (
      <div
        className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
        style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
        data-testid="design-preview-loading"
      >
        <Loader2 className="w-4 h-4 animate-spin" style={{ color: "var(--accent-primary)" }} />
        <span className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
          Loading preview
        </span>
      </div>
    );
  }

  return (
    <PreviewSample
      item={item}
      preview={preview}
      previewKind={previewKind}
      sourceCount={sourceCount}
    />
  );
}

function PreviewSample({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  if (previewKind === "color_swatch") {
    return <ColorPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "typography_sample") {
    return <TypographyPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "spacing_sample") {
    return <SpacingPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "layout_sample" || previewKind === "screen_artifact_preview") {
    return <LayoutPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  if (previewKind === "asset_sample") {
    return <AssetPreview item={item} preview={preview} sourceCount={sourceCount} />;
  }
  return <ComponentPreview item={item} preview={preview} previewKind={previewKind} sourceCount={sourceCount} />;
}

type PreviewRecord = Record<string, unknown>;

function previewRecords(preview: DesignPreviewContent | undefined, key: string): PreviewRecord[] {
  const value = (preview as PreviewRecord | undefined)?.[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is PreviewRecord => Boolean(entry) && typeof entry === "object" && !Array.isArray(entry));
}

function previewStringArray(preview: DesignPreviewContent | undefined, key: string): string[] {
  const value = (preview as PreviewRecord | undefined)?.[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is string => typeof entry === "string" && entry.trim().length > 0);
}

function recordString(record: PreviewRecord, key: string): string | null {
  const value = record[key];
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : null;
}

function sourceLabelsForPreview(
  item: DesignStyleguideItem,
  preview: DesignPreviewContent | undefined,
  fallbackCount = 4,
): string[] {
  const labels = previewStringArray(preview, "source_labels");
  if (labels.length > 0) {
    return labels.slice(0, fallbackCount);
  }
  const paths = previewStringArray(preview, "source_paths");
  const pathLabels = paths.map(labelFromSourcePath).filter(Boolean);
  if (pathLabels.length > 0) {
    return pathLabels.slice(0, fallbackCount);
  }
  const itemLabels = item.sourceRefs.map((sourceRef) => labelFromSourcePath(sourceRef.path));
  return itemLabels.length > 0 ? itemLabels.slice(0, fallbackCount) : [item.label];
}

function labelFromSourcePath(path: string): string {
  const filename = path.split("/").pop() ?? path;
  const stem = filename.replace(/\.[^.]+$/, "");
  return stem
    .replace(/[-_.]+/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/\b\w/g, (match) => match.toUpperCase())
    .trim();
}

function SourceBackedPreview({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  const labels = sourceLabelsForPreview(item, preview, 6);
  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-source-backed-preview"
    >
      <div className="grid gap-2 sm:grid-cols-2">
        {labels.map((label) => (
          <div
            key={label}
            className="rounded-md border px-2.5 py-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div className="text-[11px] font-medium" style={{ color: "var(--text-primary)" }}>
              {label}
            </div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={previewKind} sourceCount={sourceCount} />
      <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function ColorPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const swatches = previewRecords(preview, "swatches")
    .map((swatch, index) => ({
      label: recordString(swatch, "label") ?? `Source ${index + 1}`,
      value: recordString(swatch, "value"),
    }))
    .filter((swatch): swatch is { label: string; value: string } => Boolean(swatch.value));

  if (swatches.length === 0) {
    return (
      <SourceBackedPreview
        item={item}
        preview={preview}
        previewKind={preview?.preview_kind ?? "color_swatch"}
        sourceCount={sourceCount}
      />
    );
  }

  return (
    <div className="space-y-2" data-testid="design-color-preview">
      <div className="grid grid-cols-2 gap-2">
        {swatches.map((swatch) => (
          <div
            key={swatch.label}
            className="min-h-20 rounded-lg border p-2"
            style={{
              borderColor: "var(--overlay-weak)",
              background: swatch.value,
              color: readableTextColor(swatch.value),
            }}
          >
            <div className="text-[10px] font-semibold uppercase">{swatch.label}</div>
            <div className="mt-5 text-[11px]">{swatch.value}</div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "color_swatch"} sourceCount={sourceCount} />
    </div>
  );
}

function readableTextColor(color: string): string {
  if (color.startsWith("#")) {
    const hex = color.slice(1);
    const normalized = hex.length === 3
      ? hex.split("").map((value) => `${value}${value}`).join("")
      : hex.slice(0, 6);
    const red = Number.parseInt(normalized.slice(0, 2), 16);
    const green = Number.parseInt(normalized.slice(2, 4), 16);
    const blue = Number.parseInt(normalized.slice(4, 6), 16);
    if ([red, green, blue].every(Number.isFinite)) {
      return red * 0.299 + green * 0.587 + blue * 0.114 > 150 ? "#111111" : "#ffffff";
    }
  }
  return "var(--text-primary)";
}

function TypographyPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const samples = previewRecords(preview, "typography_samples");
  const rows = samples.length
    ? samples.map((sample, index) => ({
        label: recordString(sample, "label") ?? ["Display", "Body", "Label", "Code"][index] ?? "Type",
        sample: recordString(sample, "sample") ?? item.label,
        className: [
          "text-[20px] leading-7 font-semibold",
          "text-[13px] leading-5",
          "text-[11px] leading-4 font-semibold uppercase",
          "font-mono text-[12px] leading-5",
        ][index] ?? "text-[13px] leading-5",
      }))
    : [
        { label: "Display", sample: preview?.label ?? item.label, className: "text-[20px] leading-7 font-semibold" },
        { label: "Body", sample: preview?.summary ?? item.summary, className: "text-[13px] leading-5" },
        { label: "Label", sample: item.confidence.toUpperCase(), className: "text-[11px] leading-4 font-semibold uppercase" },
        { label: "Code", sample: item.sourceRefs[0]?.path ?? item.itemId, className: "font-mono text-[12px] leading-5" },
      ];
  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-typography-preview"
    >
      <div className="space-y-1">
        <div className="text-[28px] leading-9 font-semibold" style={{ color: "var(--text-primary)" }}>
          {preview?.label ?? item.label}
        </div>
        <div className="text-[13px] leading-5" style={{ color: "var(--text-secondary)" }}>
          {preview?.summary ?? item.summary}
        </div>
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        {rows.map((row) => (
          <div
            key={row.label}
            className="rounded-md border px-2.5 py-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
              {row.label}
            </div>
            <div className={row.className} style={{ color: "var(--text-primary)" }}>
              {row.sample}
            </div>
          </div>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "typography_sample"} sourceCount={sourceCount} />
    </div>
  );
}

function ComponentPreview({
  item,
  preview,
  previewKind,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  previewKind: string;
  sourceCount: number;
}) {
  const componentSamples = previewRecords(preview, "component_samples")
    .map((sample) => recordString(sample, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = componentSamples.length > 0
    ? componentSamples
    : sourceLabelsForPreview(item, preview, 4);
  const stateLabels = ["default", "hover", "focus", "loading"];

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-component-preview"
    >
      <div className="flex flex-wrap items-center gap-2">
        {labels.map((label, index) => (
          <button
            key={`${label}-${index}`}
            type="button"
            className="h-8 rounded-md border px-3 text-[12px] font-medium"
            style={{
              borderColor: index === 0 ? "var(--accent-border)" : "var(--overlay-weak)",
              color: index === 0 ? "var(--accent-primary)" : "var(--text-primary)",
              background: index === 0 ? "var(--accent-muted)" : "var(--bg-base)",
            }}
          >
            {label}
          </button>
        ))}
      </div>
      <div
        className="rounded-md border p-2"
        style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
      >
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {stateLabels.map((stateLabel) => (
            <div
              key={stateLabel}
              className="rounded-md border px-2 py-1.5 text-[11px]"
              style={{ borderColor: "var(--overlay-weak)", color: "var(--text-secondary)", background: "var(--bg-surface)" }}
            >
              {stateLabel}
            </div>
          ))}
        </div>
      </div>
      <div className="flex items-center justify-between gap-3 text-[12px]">
        <div className="min-w-0">
          <div className="font-medium truncate" style={{ color: "var(--text-primary)" }}>
            {preview?.label ?? item.label}
          </div>
          <div className="truncate" style={{ color: "var(--text-secondary)" }}>
            {preview?.summary ?? item.summary}
          </div>
        </div>
        <span
          className="shrink-0 rounded-full border px-2 py-1 text-[11px]"
          style={{ borderColor: "var(--overlay-faint)", color: "var(--text-secondary)" }}
        >
          {item.confidence} confidence
        </span>
      </div>
      <PreviewMeta previewKind={previewKind} sourceCount={sourceCount} />
    </div>
  );
}

function SpacingPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const regions = previewRecords(preview, "layout_regions")
    .map((region) => recordString(region, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = regions.length > 0 ? regions.slice(0, 3) : sourceLabelsForPreview(item, preview, 3);

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-spacing-preview"
    >
      <div className="grid grid-cols-4 gap-2">
        {[4, 8, 12, 16].map((size) => (
          <div
            key={size}
            className="rounded-md border p-2"
            style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}
          >
            <div
              className="rounded-sm"
              style={{
                height: `${Math.max(size, 8)}px`,
                background: "var(--accent-muted)",
              }}
            />
            <div className="mt-2 text-[11px]" style={{ color: "var(--text-secondary)" }}>
              {size}px
            </div>
          </div>
        ))}
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        <div className="h-14 rounded-lg border" style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-base)" }} />
        <div className="h-14 rounded-lg border shadow-lg" style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }} />
      </div>
      <div className="flex flex-wrap gap-2">
        {labels.map((label) => (
          <span
            key={label}
            className="rounded-md border px-2 py-1 text-[11px]"
            style={{
              borderColor: "var(--overlay-faint)",
              background: "var(--bg-base)",
              color: "var(--text-secondary)",
            }}
          >
            {label}
          </span>
        ))}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "spacing_sample"} sourceCount={sourceCount} />
      <div className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function LayoutPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const regions = previewRecords(preview, "layout_regions")
    .map((region) => recordString(region, "label"))
    .filter((label): label is string => Boolean(label));
  const labels = regions.length > 0 ? regions.slice(0, 3) : sourceLabelsForPreview(item, preview, 3);

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-layout-preview"
    >
      <div
        className="grid h-36 overflow-hidden rounded-md border"
        style={{
          borderColor: "var(--overlay-faint)",
          gridTemplateColumns: labels.length >= 3 ? "0.8fr 1.4fr 1fr" : "1fr 1fr",
        }}
        data-testid="design-workspace-surface-preview"
      >
        {labels.map((label, index) => (
          <div
            key={`${label}-${index}`}
            className={index < labels.length - 1 ? "border-r p-2 space-y-2" : "p-2 space-y-2"}
            style={{ borderColor: "var(--overlay-faint)", background: index % 2 === 0 ? "var(--bg-base)" : "transparent" }}
          >
            <div className="text-[10px] font-semibold uppercase" style={{ color: "var(--text-muted)" }}>
              {label}
            </div>
            <div className="h-3 w-16 rounded-sm" style={{ background: "var(--overlay-weak)" }} />
            <div className="h-6 rounded-md" style={{ background: index === 0 ? "var(--accent-muted)" : "var(--overlay-faint)" }} />
            <div className="h-6 rounded-md border" style={{ borderColor: "var(--overlay-faint)" }} />
          </div>
        ))}
      </div>
      <div className="text-[12px] font-medium" style={{ color: "var(--text-primary)" }}>
        {preview?.label ?? item.label}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "layout_sample"} sourceCount={sourceCount} />
      <div className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
    </div>
  );
}

function AssetPreview({
  item,
  preview,
  sourceCount,
}: {
  item: DesignStyleguideItem;
  preview: DesignPreviewContent | undefined;
  sourceCount: number;
}) {
  const assets = previewRecords(preview, "asset_samples")
    .map((asset) => ({
      label: recordString(asset, "label"),
      path: recordString(asset, "path"),
    }))
    .filter((asset): asset is { label: string; path: string | null } => Boolean(asset.label));
  const labels = assets.length > 0
    ? assets
    : sourceLabelsForPreview(item, preview, 4).map((label) => ({ label, path: null }));

  return (
    <div
      className="rounded-lg border p-3 space-y-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-asset-preview"
    >
      <div className="grid gap-2 sm:grid-cols-2">
        {labels.map((asset) => (
          <div key={asset.label} className="flex items-center gap-3 rounded-md border p-2" style={{ borderColor: "var(--overlay-faint)", background: "var(--bg-base)" }}>
            <div
              className="flex h-10 w-10 items-center justify-center rounded-lg border"
              style={{ borderColor: "var(--accent-border)", background: "var(--accent-muted)", color: "var(--accent-primary)" }}
            >
              <Sparkles className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <div className="truncate text-[12px] font-semibold" style={{ color: "var(--text-primary)" }}>
                {asset.label}
              </div>
              {asset.path ? (
                <div className="truncate text-[11px]" style={{ color: "var(--text-muted)" }}>
                  {asset.path}
                </div>
              ) : null}
            </div>
          </div>
        ))}
      </div>
      <div className="text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        {preview?.summary ?? item.summary}
      </div>
      <PreviewMeta previewKind={preview?.preview_kind ?? "asset_sample"} sourceCount={sourceCount} />
    </div>
  );
}

function PreviewMeta({ previewKind, sourceCount }: { previewKind: string; sourceCount: number }) {
  return (
    <div className="text-[11px]" style={{ color: "var(--text-muted)" }} data-testid="design-preview-kind">
      {formatPreviewKind(previewKind)} / {sourceCount} {sourceCount === 1 ? "source" : "sources"}
    </div>
  );
}

function previewKindForItem(item: DesignStyleguideItem): string {
  switch (item.group) {
    case "colors":
      return "color_swatch";
    case "type":
      return "typography_sample";
    case "spacing":
      return "spacing_sample";
    case "ui_kit":
      return "layout_sample";
    case "brand":
      return "asset_sample";
    case "components":
    default:
      return "component_sample";
  }
}

function formatPreviewKind(previewKind: string): string {
  return previewKind.replace(/_/g, " ");
}
