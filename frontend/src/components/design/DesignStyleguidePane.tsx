import { Check, ChevronDown, ChevronRight, MessageSquare, Package, Upload } from "lucide-react";
import { useMemo, useState } from "react";

import { Button } from "@/components/ui/button";
import type { DesignReviewState, DesignStyleguideItem, DesignSystem } from "./designSystems";
import {
  useApproveDesignStyleguideItem,
  useCreateDesignStyleguideFeedback,
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
}

export function DesignStyleguidePane({ designSystem }: DesignStyleguidePaneProps) {
  const [activeFilter, setActiveFilter] = useState<"all" | DesignReviewState>("all");
  const [expandedItemId, setExpandedItemId] = useState<string | null>(null);
  const [feedbackItemId, setFeedbackItemId] = useState<string | null>(null);
  const [approvalOverrides, setApprovalOverrides] = useState<Record<string, DesignReviewState>>({});
  const [feedbackDraft, setFeedbackDraft] = useState("");
  const approveMutation = useApproveDesignStyleguideItem();
  const feedbackMutation = useCreateDesignStyleguideFeedback();

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
        <Button type="button" variant="outline" size="sm" className="gap-2">
          <Upload className="w-4 h-4" />
          Export
        </Button>
      </header>

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <section className="space-y-2">
          <div className="text-[13px] leading-5" style={{ color: "var(--text-secondary)" }}>
            {designSystem.readySummary}
          </div>
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
                  />
                ))}
              </div>
            </section>
          );
        })}
      </div>
    </aside>
  );
}

function StyleguideRow({
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
}: {
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
}) {
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
          <PreviewBlock item={item} />
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

function PreviewBlock({ item }: { item: DesignStyleguideItem }) {
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
