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
import {
  MessageSquare,
  Plus,
  Sparkles,
  Pencil,
  Zap,
  CheckCircle,
  CircleCheck,
  Archive,
} from "lucide-react";
import type { IdeationSessionWithProgress } from "@/types/ideation";
import { ideationApi } from "@/api/ideation";
import { PlanItem } from "./PlanItem";
import { SessionGroupHeader } from "./SessionGroupHeader";
import { SessionGroupSkeleton } from "./SessionGroupSkeleton";
import { GROUP_KEY_TO_API, type SessionGroup } from "./planBrowserUtils";
import { useSessionGroupCounts } from "@/hooks/useIdeation";
import { useInfiniteSessionsQuery, flattenSessionPages } from "@/hooks/useInfiniteSessionsQuery";

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
  { key: "in-progress", label: "In Progress", icon: Zap, accentColor: "hsl(14 100% 60%)" },
  { key: "accepted", label: "Accepted", icon: CheckCircle, accentColor: "hsl(145 70% 45%)" },
  { key: "done", label: "Done", icon: CircleCheck, accentColor: "hsl(220 10% 45%)" },
  { key: "archived", label: "Archived", icon: Archive, accentColor: "hsl(220 10% 45%)" },
];

// ============================================================================
// Per-Group Section Component
// ============================================================================

interface GroupSectionProps {
  groupKey: SessionGroup;
  projectId: string;
  isOpen: boolean;
  onToggle: (open: boolean) => void;
  icon: typeof Pencil;
  label: string;
  accentColor?: string;
  count: number;
  renderItem: (plan: IdeationSessionWithProgress, group: SessionGroup) => React.ReactNode;
}

function GroupSection({
  groupKey,
  projectId,
  isOpen,
  onToggle,
  icon,
  label,
  accentColor,
  count,
  renderItem,
}: GroupSectionProps) {
  const apiKey = GROUP_KEY_TO_API[groupKey];
  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
  } = useInfiniteSessionsQuery(projectId, apiKey, { enabled: isOpen });

  const sessions = flattenSessionPages(data);

  // Intersection observer for infinite scroll
  const sentinelRef = useRef<HTMLDivElement | null>(null);
  const fetchNextPageRef = useRef(fetchNextPage);
  const hasNextPageRef = useRef(hasNextPage);
  useEffect(() => {
    fetchNextPageRef.current = fetchNextPage;
    hasNextPageRef.current = hasNextPage;
  }, [fetchNextPage, hasNextPage]);

  useEffect(() => {
    if (!sentinelRef.current || !isOpen) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const first = entries[0];
        if (first?.isIntersecting && hasNextPageRef.current && !isFetchingNextPage) {
          fetchNextPageRef.current();
        }
      },
      { threshold: 0.1 }
    );

    observer.observe(sentinelRef.current);
    return () => observer.disconnect();
  }, [isOpen, isFetchingNextPage]);

  if (count === 0) return null;

  // Drafts group renders flat (no collapsible header)
  if (groupKey === "drafts") {
    return (
      <div className="space-y-1">
        {isLoading ? (
          <SessionGroupSkeleton count={Math.min(count, 3)} />
        ) : (
          <>
            {sessions.map((plan) => renderItem(plan, groupKey))}
            {hasNextPage && (
              <div ref={sentinelRef} className="h-2" />
            )}
            {isFetchingNextPage && (
              <SessionGroupSkeleton count={1} />
            )}
          </>
        )}
      </div>
    );
  }

  return (
    <SessionGroupHeader
      icon={icon}
      label={label}
      count={count}
      isOpen={isOpen}
      onToggle={onToggle}
      {...(accentColor != null && { accentColor })}
    >
      {isOpen && (
        <>
          {isLoading ? (
            <SessionGroupSkeleton count={Math.min(count, 3)} />
          ) : (
            <>
              {sessions.map((plan) => renderItem(plan, groupKey))}
              {hasNextPage && (
                <div ref={sentinelRef} className="h-2" />
              )}
              {isFetchingNextPage && (
                <SessionGroupSkeleton count={1} />
              )}
            </>
          )}
        </>
      )}
    </SessionGroupHeader>
  );
}

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
}: PlanBrowserProps) {
  const { data: counts } = useSessionGroupCounts(projectId);

  const totalCount = counts
    ? counts.drafts + counts.inProgress + counts.accepted + counts.done + counts.archived
    : 0;

  // Default expand state: all collapsed except In Progress (if count > 0)
  const [groupOpen, setGroupOpen] = useState<Record<SessionGroup, boolean>>(() => ({
    drafts: true, // always show drafts flat (no header toggle)
    "in-progress": false, // will be updated once counts load
    accepted: false,
    done: false,
    archived: false,
  }));

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

  const [editingPlanId, setEditingPlanId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState("");
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

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

  return (
    <div
      data-testid="plan-browser"
      className="flex flex-col h-full"
      style={{
        width: "340px",
        minWidth: "340px",
        flexShrink: 0,
      }}
    >
      {/* Floating panel inner container */}
      <div
        className="flex flex-col h-full rounded-[10px]"
        style={{
          margin: "8px",
          background: "hsla(220 10% 10% / 0.92)",
          backdropFilter: "blur(20px) saturate(180%)",
          WebkitBackdropFilter: "blur(20px) saturate(180%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          boxShadow: "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
        }}
      >
        {/* Header */}
        <div
          className="px-4 pt-4 pb-3"
          style={{
            borderBottom: "1px solid hsla(220 10% 100% / 0.04)",
          }}
        >
          {/* Title */}
          <div className="flex items-center gap-2.5 mb-4">
            <div
              className="w-8 h-8 rounded-[10px] flex items-center justify-center"
              style={{
                background: "hsla(14 100% 60% / 0.12)",
                border: "1px solid hsla(14 100% 60% / 0.2)",
              }}
            >
              <Sparkles className="w-4 h-4" style={{ color: "hsl(14 100% 60%)" }} />
            </div>
            <div>
              <h2
                className="text-[13px] font-semibold tracking-[-0.01em]"
                style={{ color: "hsl(220 10% 90%)" }}
              >
                Plans
              </h2>
              <p
                className="text-[11px] tracking-[-0.005em]"
                style={{ color: "hsl(220 10% 50%)" }}
              >
                {totalCount} {totalCount === 1 ? "plan" : "plans"}
              </p>
            </div>
          </div>

          {/* New Plan Button - flat Tahoe style */}
          <Button
            onClick={onNewPlan}
            className="w-full h-9 text-[13px] font-medium tracking-[-0.01em] border-0 transition-colors duration-150"
            style={{
              background: "hsl(14 100% 60%)",
              color: "white",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 55%)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = "hsl(14 100% 60%)";
            }}
          >
            <Plus className="w-4 h-4 mr-1.5" strokeWidth={2.5} />
            New Plan
          </Button>
        </div>

        {/* Plan List */}
        <div className="flex-1 overflow-y-auto px-2 py-2">
          {!hasAnySessions ? (
            <div className="flex flex-col items-center justify-center h-full px-4 text-center">
              <div
                className="w-12 h-12 rounded-2xl flex items-center justify-center mb-3"
                style={{
                  background: "hsla(220 10% 100% / 0.03)",
                  border: "1px solid hsla(220 10% 100% / 0.06)",
                }}
              >
                <MessageSquare className="w-5 h-5" style={{ color: "hsl(220 10% 50%)" }} />
              </div>
              <p className="text-[13px] font-medium" style={{ color: "hsl(220 10% 70%)" }}>
                No plans yet
              </p>
              <p className="text-[11px] mt-1" style={{ color: "hsl(220 10% 50%)" }}>
                Start your first brainstorm
              </p>
            </div>
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
