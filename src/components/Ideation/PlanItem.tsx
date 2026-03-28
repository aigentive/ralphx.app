/**
 * PlanItem - Individual session item in the PlanBrowser sidebar
 *
 * Renders group-specific metadata below the title and context menu
 * actions appropriate for each SessionGroup.
 */

import { memo, useCallback } from "react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
  MessageSquare,
  Loader2,
  Clock,
  MoreHorizontal,
  Pencil,
  Archive,
  RotateCcw,
  RefreshCw,
  CircleCheck,
  CornerDownRight,
  ArrowDownToLine,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useChatStore, selectAgentStatus } from "@/stores/chatStore";
import { useIdeationStore } from "@/stores/ideationStore";
import { buildStoreKey } from "@/lib/chat-context-registry";
import type { IdeationSessionWithProgress, SessionProgress } from "@/types/ideation";
import type { SessionGroup } from "./planBrowserUtils";

// ============================================================================
// Helpers
// ============================================================================

function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function formatDate(dateString: string): string {
  try {
    const date = new Date(dateString);
    return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  } catch {
    return "";
  }
}

// ============================================================================
// ActivityIndicator
// ============================================================================

const ACCENT_COLOR = "hsl(14 100% 60%)";
const VERIFYING_COLOR = "hsl(217 91% 60%)";

interface ActivityIndicatorProps {
  isActive: boolean;
  isWaiting: boolean;
  label?: string | undefined;
  separator?: string | undefined;
  color?: string | undefined;
}

function ActivityIndicator({ isActive, isWaiting, label = "Agent working...", separator, color = ACCENT_COLOR }: ActivityIndicatorProps) {
  if (!isActive && !isWaiting) return null;
  return (
    <>
      {isActive && <span style={{ color }}>{label}</span>}
      {isWaiting && <span style={{ color: "hsl(220 10% 45%)" }}>Awaiting input</span>}
      {separator && <span style={{ color: "hsl(220 10% 35%)" }}>{separator}</span>}
    </>
  );
}

// ============================================================================
// Types
// ============================================================================

export interface PlanItemProps {
  plan: IdeationSessionWithProgress;
  isSelected: boolean;
  group: SessionGroup;
  isEditing: boolean;
  editingTitle?: string | undefined;
  isMenuOpen: boolean;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onSelect: (planId: string) => void;
  onStartRename: (planId: string, currentTitle: string) => void;
  onConfirmRename: (planId: string) => void;
  onTitleChange: (value: string) => void;
  onKeyDown: (e: React.KeyboardEvent, planId: string) => void;
  onMenuOpenChange: (open: boolean, planId: string) => void;
  onArchive?: (planId: string) => void;
  onReopen?: (planId: string) => void;
  onResetReaccept?: (planId: string) => void;
  onNavigateToSource?: (planId: string) => void;
}

// ============================================================================
// Metadata Line
// ============================================================================

interface MetadataLineProps {
  group: SessionGroup;
  plan: IdeationSessionWithProgress;
  progress: SessionProgress | null;
  isIdeationActive: boolean;
  isIdeationWaiting: boolean;
  isVerifying: boolean;
}

function MetadataLine({ group, plan, progress, isIdeationActive, isIdeationWaiting, isVerifying }: MetadataLineProps) {
  const parentSessionTitle = plan.parentSessionTitle;
  const activityLabel = isVerifying ? "Verifying..." : undefined;
  const activityColor = isVerifying ? VERIFYING_COLOR : undefined;

  // Show parent session indicator if this is a child session
  if (parentSessionTitle) {
    return (
      <div
        className="flex flex-col gap-0.5 text-[10px]"
        style={{ color: "hsl(220 10% 45%)" }}
      >
        <ActivityIndicator isActive={isIdeationActive} isWaiting={isIdeationWaiting} label={activityLabel} color={activityColor} />
        <div className="flex items-center gap-1">
          <CornerDownRight className="w-2.5 h-2.5" />
          <span className="truncate">Follow-up of: {parentSessionTitle}</span>
        </div>
      </div>
    );
  }

  switch (group) {
    case "drafts":
      return (
        <div
          className="flex items-center gap-1 text-[10px]"
          style={{ color: "hsl(220 10% 45%)" }}
        >
          {(isIdeationActive || isIdeationWaiting) ? (
            <ActivityIndicator isActive={isIdeationActive} isWaiting={isIdeationWaiting} label={activityLabel} color={activityColor} />
          ) : (
            <>
              <Clock className="w-2.5 h-2.5" />
              <span>{formatRelativeTime(plan.updatedAt)}</span>
            </>
          )}
        </div>
      );

    case "in-progress":
      if (!progress) {
        if (isIdeationActive || isIdeationWaiting) {
          return (
            <span className="text-[10px]">
              <ActivityIndicator isActive={isIdeationActive} isWaiting={isIdeationWaiting} label={activityLabel} color={activityColor} />
            </span>
          );
        }
        return null;
      }
      return (
        <div className="flex items-center gap-1 text-[10px]">
          <ActivityIndicator isActive={isIdeationActive} isWaiting={isIdeationWaiting} label={activityLabel ?? "Agent working"} separator="·" color={activityColor} />
          <span style={{ color: "hsl(145 70% 50%)" }}>
            {progress.done}/{progress.total} done
          </span>
          {progress.active > 0 && (
            <>
              <span style={{ color: "hsl(220 10% 35%)" }}>&middot;</span>
              <span style={{ color: "hsl(14 100% 60%)" }}>
                {progress.active} active
              </span>
            </>
          )}
        </div>
      );

    case "accepted":
      return (
        <div
          className="flex items-center gap-1 text-[10px]"
          style={{ color: "hsl(220 10% 45%)" }}
        >
          <ActivityIndicator isActive={isIdeationActive} isWaiting={isIdeationWaiting} separator="·" label={activityLabel} color={activityColor} />
          <span>{progress?.total ?? 0} {(progress?.total ?? 0) === 1 ? "task" : "tasks"}</span>
          {plan.convertedAt && (
            <>
              <span>&middot;</span>
              <span>{formatDate(plan.convertedAt)}</span>
            </>
          )}
        </div>
      );

    case "done":
      return (
        <div
          className="flex items-center gap-1 text-[10px]"
          style={{ color: "hsl(220 10% 40%)" }}
        >
          <ActivityIndicator isActive={isIdeationActive} isWaiting={isIdeationWaiting} separator="·" label={activityLabel} color={activityColor} />
          <CircleCheck className="w-2.5 h-2.5" style={{ color: "hsl(145 70% 40%)" }} />
          <span>Completed</span>
        </div>
      );

    case "archived":
      return (
        <div
          className="flex items-center gap-1 text-[10px]"
          style={{ color: "hsl(220 10% 40%)" }}
        >
          <ActivityIndicator isActive={isIdeationActive} isWaiting={isIdeationWaiting} separator="·" label={activityLabel} color={activityColor} />
          {plan.archivedAt ? (
            <span>Archived {formatDate(plan.archivedAt)}</span>
          ) : (
            <span>Archived</span>
          )}
        </div>
      );
  }
}

// ============================================================================
// Context Menu
// ============================================================================

const MENU_ITEM_CLASSES = "text-[13px] cursor-pointer gap-2.5 py-2";

function ContextMenuItems({ group, onStartRename, onArchive, onReopen, onResetReaccept }: {
  group: SessionGroup;
  onStartRename: () => void;
  onArchive?: () => void;
  onReopen?: () => void;
  onResetReaccept?: () => void;
}) {
  return (
    <>
      {/* Rename is available for all groups */}
      <DropdownMenuItem
        onClick={(e) => { e.stopPropagation(); onStartRename(); }}
        className={MENU_ITEM_CLASSES}
      >
        <Pencil className="w-3.5 h-3.5" />
        Rename
      </DropdownMenuItem>

      {group === "drafts" && (
        <DropdownMenuItem
          onClick={(e) => { e.stopPropagation(); onArchive?.(); }}
          className={MENU_ITEM_CLASSES}
        >
          <Archive className="w-3.5 h-3.5" />
          Archive
        </DropdownMenuItem>
      )}

      {(group === "in-progress" || group === "accepted" || group === "done") && (
        <>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onReopen?.(); }}
            className={MENU_ITEM_CLASSES}
          >
            <RotateCcw className="w-3.5 h-3.5" />
            Reopen
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onResetReaccept?.(); }}
            className={MENU_ITEM_CLASSES}
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Reset & Re-accept
          </DropdownMenuItem>
        </>
      )}

      {group === "archived" && (
        <DropdownMenuItem
          onClick={(e) => { e.stopPropagation(); onReopen?.(); }}
          className={MENU_ITEM_CLASSES}
        >
          <RotateCcw className="w-3.5 h-3.5" />
          Reopen
        </DropdownMenuItem>
      )}
    </>
  );
}

// ============================================================================
// Component
// ============================================================================

const isMutedGroup = (group: SessionGroup) => group === "done" || group === "archived";

export const PlanItem = memo(function PlanItem({
  plan,
  isSelected,
  group,
  isEditing,
  editingTitle,
  isMenuOpen,
  inputRef,
  onSelect,
  onStartRename,
  onConfirmRename,
  onTitleChange,
  onKeyDown,
  onMenuOpenChange,
  onArchive,
  onReopen,
  onResetReaccept,
  onNavigateToSource,
}: PlanItemProps) {
  const muted = isMutedGroup(group);
  const storeKey = buildStoreKey("ideation", plan.id);
  const agentStatus = useChatStore(selectAgentStatus(storeKey));
  const isIdeationActive = agentStatus === "generating";
  const isIdeationWaiting = agentStatus === "waiting_for_input";
  const activeVerificationChildId = useIdeationStore(state => state.activeVerificationChildId[plan.id]);
  const isVerifying = isIdeationActive && !!activeVerificationChildId;

  // Internal stable handlers — bind planId so parent callbacks stay stable
  const handleClick = useCallback(() => {
    if (!isEditing) onSelect(plan.id);
  }, [onSelect, plan.id, isEditing]);

  const handleStartRename = useCallback(() => {
    onStartRename(plan.id, plan.title ?? "");
  }, [onStartRename, plan.id, plan.title]);

  const handleConfirmRename = useCallback(() => {
    onConfirmRename(plan.id);
  }, [onConfirmRename, plan.id]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    onKeyDown(e, plan.id);
  }, [onKeyDown, plan.id]);

  const handleMenuOpenChange = useCallback((open: boolean) => {
    onMenuOpenChange(open, plan.id);
  }, [onMenuOpenChange, plan.id]);

  const handleArchive = useCallback(() => {
    onArchive?.(plan.id);
  }, [onArchive, plan.id]);

  const handleReopen = useCallback(() => {
    onReopen?.(plan.id);
  }, [onReopen, plan.id]);

  const handleResetReaccept = useCallback(() => {
    onResetReaccept?.(plan.id);
  }, [onResetReaccept, plan.id]);

  const handleNavigateToSource = useCallback(() => {
    onNavigateToSource?.(plan.id);
  }, [onNavigateToSource, plan.id]);

  return (
    <div
      data-testid={`plan-item-${plan.id}`}
      className={cn(
        "group relative rounded-md cursor-pointer",
        "transition-all duration-150 ease-out"
      )}
      style={{
        padding: "6px 8px",
        background: isSelected
          ? "hsla(14 100% 60% / 0.12)"
          : isMenuOpen
            ? "hsla(220 10% 100% / 0.04)"
            : "transparent",
        border: isSelected
          ? "1px solid hsla(14 100% 60% / 0.2)"
          : "1px solid transparent",
        opacity: muted && !isSelected ? 0.7 : 1,
      }}
      onClick={handleClick}
      onMouseEnter={(e) => {
        if (!isSelected && !isMenuOpen) {
          e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
        }
      }}
      onMouseLeave={(e) => {
        if (!isSelected && !isMenuOpen) {
          e.currentTarget.style.background = "transparent";
        }
      }}
    >
      <div className="flex items-center gap-2">
        {/* Plan icon */}
        <div
          className="w-6 h-6 rounded-md flex items-center justify-center flex-shrink-0 transition-colors duration-150"
          style={{
            background: isSelected
              ? "hsla(14 100% 60% / 0.15)"
              : "hsla(220 10% 100% / 0.04)",
            border: isSelected
              ? "1px solid hsla(14 100% 60% / 0.2)"
              : "1px solid hsla(220 10% 100% / 0.06)",
          }}
        >
          {isIdeationActive ? (
            <Loader2
              className="w-3 h-3 animate-spin"
              style={{ color: isVerifying ? VERIFYING_COLOR : ACCENT_COLOR }}
            />
          ) : (
            <MessageSquare
              className="w-3 h-3"
              style={{ color: isIdeationWaiting || isSelected ? "hsl(14 100% 60%)" : "hsl(220 10% 50%)" }}
            />
          )}
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {isEditing ? (
            <Input
              ref={inputRef}
              value={editingTitle ?? ""}
              onChange={(e) => onTitleChange(e.target.value)}
              onKeyDown={handleKeyDown}
              onBlur={handleConfirmRename}
              className="h-6 text-[13px] px-2 py-0 rounded-md"
              style={{
                background: "hsl(220 10% 12%)",
                border: "1px solid hsla(220 10% 100% / 0.1)",
              }}
              onClick={(e) => e.stopPropagation()}
            />
          ) : (
            <>
              <div className="flex items-center gap-1.5">
                <span
                  className={cn(
                    "text-[12px] font-medium truncate tracking-[-0.01em]",
                    "transition-colors duration-150"
                  )}
                  style={{
                    color: isSelected
                      ? "hsl(220 10% 90%)"
                      : muted
                        ? "hsl(220 10% 55%)"
                        : "hsl(220 10% 70%)",
                  }}
                >
                  {plan.title || "Untitled Plan"}
                </span>
                {plan.sourceProjectId && (
                  <button
                    type="button"
                    data-testid="import-badge"
                    className="inline-flex items-center gap-0.5 text-[9px] font-medium px-1 py-0.5 rounded flex-shrink-0 select-none transition-opacity hover:opacity-80"
                    style={{
                      background: "hsla(145 70% 45% / 0.1)",
                      border: "1px solid hsla(145 70% 45% / 0.25)",
                      color: "hsl(145 70% 50%)",
                    }}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleNavigateToSource();
                    }}
                    title="Navigate to source session"
                  >
                    <ArrowDownToLine className="w-2 h-2" />
                    Imported
                  </button>
                )}
                {plan.hasPendingPrompt && (
                  <span
                    className="inline-flex items-center justify-center w-2 h-2 rounded-full bg-amber-400 animate-pulse flex-shrink-0"
                    title="Waiting for capacity — message queued"
                  />
                )}
              </div>
              <MetadataLine
                group={group}
                plan={plan}
                progress={plan.progress ?? null}
                isIdeationActive={isIdeationActive}
                isIdeationWaiting={isIdeationWaiting}
                isVerifying={isVerifying}
              />
            </>
          )}
        </div>

        {/* Context Menu */}
        {!isEditing && (
          <DropdownMenu onOpenChange={handleMenuOpenChange}>
            <DropdownMenuTrigger asChild>
              <button
                className={cn(
                  "w-6 h-6 rounded flex items-center justify-center flex-shrink-0",
                  "transition-all duration-150",
                  (isMenuOpen || isSelected)
                    ? "opacity-100"
                    : "opacity-0 group-hover:opacity-100"
                )}
                style={{
                  background: isMenuOpen ? "hsla(220 10% 100% / 0.08)" : "transparent",
                }}
                onClick={(e) => e.stopPropagation()}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "hsla(220 10% 100% / 0.08)";
                }}
                onMouseLeave={(e) => {
                  if (!isMenuOpen) {
                    e.currentTarget.style.background = "transparent";
                  }
                }}
              >
                <MoreHorizontal className="w-3.5 h-3.5" style={{ color: "hsl(220 10% 50%)" }} />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              align="end"
              className="w-48"
              style={{
                background: "hsl(220 10% 14%)",
                border: "1px solid hsla(220 10% 100% / 0.08)",
                boxShadow: "0 8px 32px hsla(0 0% 0% / 0.4)",
              }}
            >
              <ContextMenuItems
                group={group}
                onStartRename={handleStartRename}
                onArchive={handleArchive}
                onReopen={handleReopen}
                onResetReaccept={handleResetReaccept}
              />
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
    </div>
  );
});
