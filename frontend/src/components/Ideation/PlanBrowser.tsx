/**
 * PlanBrowser - macOS Tahoe styled sidebar for ideation plans
 *
 * Design: Native macOS sidebar with frosted glass, refined typography,
 * and smooth spring animations. Warm orange accent (#ff6b35).
 *
 * Five semantic groups: Drafts, In Progress, Accepted, Done, Archived.
 * Uses server-side paginated queries per group, lazy-loaded on expand
 * with infinite scroll. Groups default to collapsed except In Progress.
 */

import { useState, useRef, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import {
  ChevronLeft,
  MessageSquare,
  Plus,
  Search,
  X,
  Loader2,
  Lightbulb,
  Pencil,
  Zap,
  CheckCircle,
  CircleCheck,
  Archive,
} from "lucide-react";
import type { IdeationSessionWithProgress } from "@/types/ideation";
import { ideationApi } from "@/api/ideation";
import { withAlpha } from "@/lib/theme-colors";
import { PlanItem } from "./PlanItem";
import type { SessionGroup } from "./planBrowserUtils";
import { GroupSection } from "./GroupSection";
import { useSessionGroupCounts } from "@/hooks/useIdeation";
import { usePlanBrowserSearch } from "@/hooks/usePlanBrowserSearch";

// ============================================================================
// Types
// ============================================================================

interface PlanBrowserProps {
  projectId: string;
  currentPlanId: string | null;
  onSelectPlan: (planId: string) => void;
  onNewPlan: () => void;
  onArchivePlan?: (planId: string) => void;
  onReopenPlan?: (planId: string) => void;
  onResetReacceptPlan?: (planId: string) => void;
  width?: number;
  onCollapse?: () => void;
}

// ============================================================================
// Group Config
// ============================================================================

const GROUP_CONFIG: {
  key: SessionGroup;
  label: string;
  icon: typeof Pencil;
  accentColor?: string;
}[] = [
  { key: "drafts", label: "Drafts", icon: Pencil },
  { key: "in-progress", label: "In Progress", icon: Zap, accentColor: "var(--accent-primary)" },
  { key: "accepted", label: "Accepted", icon: CheckCircle, accentColor: "var(--status-success)" },
  { key: "done", label: "Done", icon: CircleCheck, accentColor: "var(--text-muted)" },
  { key: "archived", label: "Archived", icon: Archive, accentColor: "var(--text-muted)" },
];

// ============================================================================
// Component
// ============================================================================

export function PlanBrowser({
  projectId,
  currentPlanId,
  onSelectPlan,
  onNewPlan,
  onArchivePlan,
  onReopenPlan,
  onResetReacceptPlan,
  width = 340,
  onCollapse,
}: PlanBrowserProps) {
  // Default expand state: all collapsed except In Progress (if count > 0)
  const [groupOpen, setGroupOpen] = useState<Record<SessionGroup, boolean>>(() => ({
    drafts: true, // always show drafts flat (no header toggle)
    "in-progress": false, // will be updated once counts load
    accepted: false,
    done: false,
    archived: false,
  }));

  const {
    searchTerm,
    debouncedSearch,
    isSearchActive,
    isSearchLoading: isDebouncePending,
    handleSearchChange,
    handleSearchClear,
  } = usePlanBrowserSearch(groupOpen, setGroupOpen);

  const { data: counts, isFetching: isCountsFetching } = useSessionGroupCounts(projectId, debouncedSearch || undefined);

  const isSearchLoading = isDebouncePending || (isSearchActive && isCountsFetching);

  const totalCount = counts
    ? counts.drafts + counts.inProgress + counts.accepted + counts.done + counts.archived
    : 0;

  // Open In Progress automatically once counts load and inProgress > 0
  const countsLoadedRef = useRef(false);
  useEffect(() => {
    if (counts && !countsLoadedRef.current) {
      countsLoadedRef.current = true;
      if (counts.inProgress > 0) {
        setGroupOpen((prev) => ({ ...prev, "in-progress": true }));
      }
    }
  }, [counts]);

  // Auto-expand groups with matches, auto-collapse empty groups during active search
  useEffect(() => {
    if (!counts || !isSearchActive) return;

    const groupKeyToCount: Record<SessionGroup, number> = {
      drafts: counts.drafts,
      "in-progress": counts.inProgress,
      accepted: counts.accepted,
      done: counts.done,
      archived: counts.archived,
    };

    setGroupOpen((prev) => {
      const next = { ...prev };
      for (const groupKey of Object.keys(groupKeyToCount) as SessionGroup[]) {
        const count = groupKeyToCount[groupKey];
        next[groupKey] = count > 0;
      }
      return next;
    });
  }, [counts, isSearchActive]);

  const [editingPlanId, setEditingPlanId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState("");
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Keep a ref to editingTitle so confirm/keydown handlers don't close over stale state
  const editingTitleRef = useRef(editingTitle);
  useEffect(() => {
    editingTitleRef.current = editingTitle;
  }, [editingTitle]);

  useEffect(() => {
    if (editingPlanId && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingPlanId]);

  // Stable callbacks — defined at component level with planId parameter

  const handleSelect = useCallback((planId: string) => {
    onSelectPlan(planId);
  }, [onSelectPlan]);

  const handleStartRename = useCallback((planId: string, currentTitle: string) => {
    setEditingPlanId(planId);
    setEditingTitle(currentTitle);
  }, []);

  const handleCancelRename = useCallback(() => {
    setEditingPlanId(null);
    setEditingTitle("");
  }, []);

  const handleConfirmRename = useCallback(async (planId: string) => {
    const trimmedTitle = editingTitleRef.current.trim();
    if (trimmedTitle) {
      try {
        await ideationApi.sessions.updateTitle(planId, trimmedTitle);
      } catch (error) {
        console.error("Failed to rename plan:", error);
      }
    }
    setEditingPlanId(null);
    setEditingTitle("");
  }, []); // Uses ref — no editingTitle dep

  const handleKeyDown = useCallback((e: React.KeyboardEvent, planId: string) => {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleConfirmRename(planId);
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCancelRename();
    }
  }, [handleConfirmRename, handleCancelRename]);

  const handleGroupToggle = useCallback((group: SessionGroup, open: boolean) => {
    setGroupOpen((prev) => ({ ...prev, [group]: open }));
  }, []);

  const handleMenuOpenChange = useCallback((open: boolean, planId: string) => {
    setOpenMenuId(open ? planId : null);
  }, []);

  const handleArchive = useCallback((planId: string) => {
    onArchivePlan?.(planId);
  }, [onArchivePlan]);

  const handleReopen = useCallback((planId: string) => {
    onReopenPlan?.(planId);
  }, [onReopenPlan]);

  const handleResetReaccept = useCallback((planId: string) => {
    onResetReacceptPlan?.(planId);
  }, [onResetReacceptPlan]);

  const renderPlanItem = useCallback((plan: IdeationSessionWithProgress, group: SessionGroup) => (
    <PlanItem
      key={plan.id}
      plan={plan}
      isSelected={plan.id === currentPlanId}
      group={group}
      isEditing={editingPlanId === plan.id}
      editingTitle={plan.id === editingPlanId ? editingTitle : undefined}
      isMenuOpen={openMenuId === plan.id}
      inputRef={inputRef}
      onSelect={handleSelect}
      onStartRename={handleStartRename}
      onConfirmRename={handleConfirmRename}
      onTitleChange={setEditingTitle}
      onKeyDown={handleKeyDown}
      onMenuOpenChange={handleMenuOpenChange}
      onArchive={handleArchive}
      onReopen={handleReopen}
      onResetReaccept={handleResetReaccept}
    />
  ), [
    currentPlanId,
    editingPlanId,
    editingTitle,
    openMenuId,
    handleSelect,
    handleStartRename,
    handleConfirmRename,
    handleKeyDown,
    handleMenuOpenChange,
    handleArchive,
    handleReopen,
    handleResetReaccept,
  ]);

  const hasAnySessions = totalCount > 0;
  const isEmptySearchResult = isSearchActive && totalCount === 0;

  // Accessible result count announcement
  const resultCountText = isSearchActive
    ? totalCount === 0
      ? "No sessions match"
      : `${totalCount} ${totalCount === 1 ? "session" : "sessions"} found`
    : "";

  return (
    <div
      data-testid="plan-browser"
      className="flex flex-col h-full"
      style={{
        width,
        minWidth: width,
        flexShrink: 0,
      }}
    >
      {/* Panel inner container — fills to layout edges. Phase 1 region
         border on the outer plan-browser element separates the rail
         from main content, so no card stroke/gap needed here. */}
      <div
        className="flex flex-col h-full"
        style={{
          background: withAlpha("var(--bg-surface)", 92),
          backdropFilter: "blur(20px) saturate(180%)",
          WebkitBackdropFilter: "blur(20px) saturate(180%)",
        }}
      >
        {/* Header */}
        <div
          className="px-4 pt-4 pb-3"
          style={{
            borderBottom: "1px solid var(--overlay-faint)",
          }}
        >
          {/* Title */}
          <div className="flex items-center gap-2.5 mb-4">
            <div
              className="w-8 h-8 rounded-[10px] flex items-center justify-center"
              style={{
                background: withAlpha("var(--accent-primary)", 12),
                border: "1px solid var(--accent-border)",
              }}
            >
              <Lightbulb className="w-4 h-4" style={{ color: "var(--accent-primary)" }} />
            </div>
            <div>
              <h2
                className="text-[13px] font-semibold tracking-[-0.01em]"
                style={{ color: "var(--text-primary)" }}
              >
                Plans
              </h2>
              <p
                className="text-[11px] tracking-[-0.005em]"
                style={{ color: "var(--text-muted)" }}
              >
                {totalCount} {totalCount === 1 ? "plan" : "plans"}
              </p>
            </div>
            {onCollapse != null && (
              <button
                type="button"
                aria-label="Close sidebar"
                aria-expanded={true}
                data-testid="sidebar-collapse-button"
                onClick={onCollapse}
                className="ml-auto w-7 h-7 flex items-center justify-center rounded-md transition-colors duration-150"
                style={{ color: "var(--text-muted)" }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.color = "var(--text-primary)";
                  e.currentTarget.style.background = "var(--overlay-weak)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.color = "var(--text-muted)";
                  e.currentTarget.style.background = "transparent";
                }}
              >
                <ChevronLeft className="w-4 h-4" />
              </button>
            )}
          </div>

          {/* New Plan Button - flat Tahoe style */}
          <Button
            onClick={onNewPlan}
            className="w-full h-9 text-[13px] font-medium tracking-[-0.01em] border-0 transition-colors duration-150 mb-2"
            style={{
              background: "var(--accent-primary)",
              color: "var(--text-inverse)",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = withAlpha("var(--accent-primary)", 90);
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "var(--accent-primary)";
            }}
          >
            <Plus className="w-4 h-4 mr-1.5" strokeWidth={2.5} />
            New Plan
          </Button>

          {/* Search Input - Tahoe glass style */}
          <div
            className="relative flex items-center"
            style={{
              background: "var(--overlay-faint)",
              border: "1px solid var(--overlay-weak)",
              borderRadius: "6px",
            }}
          >
            <Search
              className="absolute left-2.5 w-3.5 h-3.5 pointer-events-none"
              style={{ color: "var(--text-muted)" }}
            />
            <input
              ref={searchInputRef}
              type="text"
              value={searchTerm}
              onChange={(e) => handleSearchChange(e.target.value)}
              placeholder="Search sessions..."
              aria-label="Search sessions"
              className="w-full h-8 pl-8 pr-8 text-[12px] bg-transparent outline-none ring-0 focus:ring-0 focus:outline-none focus-visible:outline-none border-0"
              style={{
                color: "var(--text-primary)",
                caretColor: "var(--accent-primary)",
              }}
            />
            {/* Right side: spinner or clear button */}
            <div className="absolute right-2 flex items-center">
              {isSearchLoading ? (
                <Loader2
                  className="w-3.5 h-3.5 animate-spin"
                  style={{ color: "var(--text-muted)" }}
                />
              ) : searchTerm !== "" ? (
                <button
                  type="button"
                  aria-label="Clear search"
                  onClick={() => {
                    handleSearchClear();
                    searchInputRef.current?.focus();
                  }}
                  className="w-4 h-4 flex items-center justify-center rounded-sm transition-colors duration-100"
                  style={{ color: "var(--text-muted)" }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.color = "var(--text-primary)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.color = "var(--text-muted)";
                  }}
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              ) : null}
            </div>
          </div>

          {/* Accessible live region for result count */}
          <div
            aria-live="polite"
            aria-atomic="true"
            className="sr-only"
          >
            {resultCountText}
          </div>
        </div>

        {/* Plan List */}
        <div className="flex-1 overflow-y-auto py-2">
          {!hasAnySessions && !isSearchActive ? (
            <EmptyState
              variant="neutral"
              icon={<MessageSquare />}
              title="No plans yet"
              description="Start your first brainstorm"
              className="h-full"
            />
          ) : isEmptySearchResult ? (
            <EmptyState
              variant="neutral"
              icon={<Search />}
              title="No sessions match"
              description="Try a different search term"
              className="h-full"
            />
          ) : (
            <>
              {GROUP_CONFIG.map(({ key, label, icon, accentColor }) => {
                const count = counts
                  ? key === "drafts"
                    ? counts.drafts
                    : key === "in-progress"
                      ? counts.inProgress
                      : key === "accepted"
                        ? counts.accepted
                        : key === "done"
                          ? counts.done
                          : counts.archived
                  : 0;

                return (
                  <GroupSection
                    key={key}
                    groupKey={key}
                    projectId={projectId}
                    isOpen={groupOpen[key]}
                    onToggle={(open) => handleGroupToggle(key, open)}
                    icon={icon}
                    label={label}
                    count={count}
                    search={debouncedSearch}
                    {...(accentColor != null && { accentColor })}
                    renderItem={renderPlanItem}
                  />
                );
              })}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
