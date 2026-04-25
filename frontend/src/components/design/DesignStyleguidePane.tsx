import { Download, Loader2, Package, Sparkles } from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import type { ExportDesignSystemPackageResponse } from "@/api/design";
import type { DesignReviewState, DesignStyleguideItem, DesignSystem } from "./designSystems";
import { FocusedItemDrawer, StyleguideRow } from "./DesignStyleguideRows";
import {
  useApproveDesignStyleguideItem,
  useCreateDesignStyleguideFeedback,
  useGenerateDesignArtifact,
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
