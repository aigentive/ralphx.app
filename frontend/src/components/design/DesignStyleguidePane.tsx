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
import type { DesignReviewState, DesignStyleguideItem, DesignSystem } from "./designSystems";
import {
  useApproveDesignStyleguideItem,
  useCreateDesignStyleguideFeedback,
  useGenerateDesignArtifact,
  useDesignStyleguidePreview,
} from "./useProjectDesignSystems";

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
  exportPackageArtifactId?: string | null;
  onGenerateStyleguide?: () => void;
  onExportPackage?: () => void;
}

export function DesignStyleguidePane({
  designSystem,
  isGeneratingStyleguide = false,
  isExportingPackage = false,
  exportPackageArtifactId = null,
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
    !!designSystem &&
    (designSystem.status === "draft" || designSystem.status === "failed" || !hasPersistedItems);

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
            {isGeneratingStyleguide ? "Generating" : "Generate"}
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
              className="rounded-lg border px-3 py-2 text-[12px]"
              style={{
                borderColor: "var(--accent-border)",
                background: "var(--accent-muted)",
                color: "var(--text-secondary)",
              }}
              data-testid="design-generating-state"
            >
              Analyzing selected sources and publishing the initial styleguide.
            </div>
          )}
          {exportPackageArtifactId && (
            <div
              className="rounded-lg border px-3 py-2 text-[12px]"
              style={{
                borderColor: "var(--overlay-weak)",
                background: "var(--bg-surface)",
                color: "var(--text-secondary)",
              }}
              data-testid="design-export-result"
            >
              Export package artifact {exportPackageArtifactId.slice(0, 8)} is ready.
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
        <span className="text-[10px] font-medium" style={{ color: "var(--text-muted)" }}>
          {reviewState.replace("_", " ")}
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
              Looks good
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
              Needs work
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

  if (!item.isPersisted) {
    return <LocalPreviewBlock item={item} />;
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

  if (item.group === "colors") {
    return (
      <div className="grid grid-cols-2 gap-2" data-testid="design-color-preview">
        {[
          preview?.label ?? item.label,
          `${preview?.preview_kind.replace("_", " ") ?? "color swatch"} / ${sourceCount} sources`,
        ].map((label, index) => (
          <div
            key={label}
            className="min-h-20 rounded-lg border p-2"
            style={{
              borderColor: "var(--overlay-weak)",
              background: index === 0 ? "var(--accent-primary)" : "var(--accent-muted)",
              color: index === 0 ? "var(--bg-base)" : "var(--accent-primary)",
            }}
          >
            <div className="text-[10px] font-semibold uppercase">{label}</div>
          </div>
        ))}
      </div>
    );
  }

  return (
    <div
      className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-component-preview"
    >
      <Package className="w-4 h-4" style={{ color: "var(--accent-primary)" }} />
      <div className="min-w-0 text-[12px] leading-5" style={{ color: "var(--text-secondary)" }}>
        <div className="font-medium truncate" style={{ color: "var(--text-primary)" }}>
          {preview?.label ?? item.label}
        </div>
        <div className="truncate" data-testid="design-preview-kind">
          {preview?.preview_kind.replace("_", " ") ?? "persisted preview"} / {sourceCount} sources
        </div>
      </div>
    </div>
  );
}

function LocalPreviewBlock({ item }: { item: DesignStyleguideItem }) {
  if (item.group === "colors") {
    return (
      <div className="grid grid-cols-2 gap-2" data-testid="design-color-preview">
        {["Primary", "Primary soft"].map((label, index) => (
          <div
            key={label}
            className="min-h-20 rounded-lg border p-2"
            style={{
              borderColor: "var(--overlay-weak)",
              background: index === 0 ? "var(--accent-primary)" : "var(--accent-muted)",
              color: index === 0 ? "var(--bg-base)" : "var(--accent-primary)",
            }}
          >
            <div className="text-[10px] font-semibold uppercase">{label}</div>
          </div>
        ))}
      </div>
    );
  }

  return (
    <div
      className="min-h-24 rounded-lg border flex items-center justify-center gap-2 px-3"
      style={{ borderColor: "var(--overlay-weak)", background: "var(--bg-surface)" }}
      data-testid="design-component-preview"
    >
      <Package className="w-4 h-4" style={{ color: "var(--accent-primary)" }} />
      <span className="text-[12px]" style={{ color: "var(--text-secondary)" }}>
        {item.previewArtifactId}
      </span>
    </div>
  );
}
