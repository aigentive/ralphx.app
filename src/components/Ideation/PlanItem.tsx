/**
 * PlanItem - Individual session item in the PlanBrowser sidebar
 *
 * Renders group-specific metadata below the title and context menu
 * actions appropriate for each SessionGroup.
 */

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
  MessageSquare,
  Clock,
  MoreHorizontal,
  Pencil,
  Archive,
  Trash2,
  RotateCcw,
  RefreshCw,
  CircleCheck,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { IdeationSession } from "@/types/ideation";
import type { SessionGroup } from "./planBrowserUtils";
import type { SessionProgress } from "@/hooks/useSessionProgress";

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
// Types
// ============================================================================

export interface PlanItemProps {
  plan: IdeationSession;
  isSelected: boolean;
  group: SessionGroup;
  progress?: SessionProgress;
  isEditing: boolean;
  editingTitle: string;
  isMenuOpen: boolean;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onSelect: () => void;
  onStartRename: () => void;
  onCancelRename: () => void;
  onConfirmRename: () => void;
  onTitleChange: (value: string) => void;
  onKeyDown: (e: React.KeyboardEvent) => void;
  onMenuOpenChange: (open: boolean) => void;
  onArchive?: () => void;
  onDelete?: () => void;
  onReopen?: () => void;
  onResetReaccept?: () => void;
}

// ============================================================================
// Metadata Line
// ============================================================================

function MetadataLine({ group, plan, progress }: {
  group: SessionGroup;
  plan: IdeationSession;
  progress?: SessionProgress;
}) {
  switch (group) {
    case "drafts":
      return (
        <div
          className="flex items-center gap-1 text-[10px]"
          style={{ color: "hsl(220 10% 45%)" }}
        >
          <Clock className="w-2.5 h-2.5" />
          <span>{formatRelativeTime(plan.updatedAt)}</span>
        </div>
      );

    case "in-progress":
      if (!progress) return null;
      return (
        <div className="flex items-center gap-1 text-[10px]">
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

function ContextMenuItems({ group, onStartRename, onArchive, onDelete, onReopen, onResetReaccept }: {
  group: SessionGroup;
  onStartRename: () => void;
  onArchive?: () => void;
  onDelete?: () => void;
  onReopen?: () => void;
  onResetReaccept?: () => void;
}) {
  switch (group) {
    case "drafts":
      return (
        <>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onStartRename(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
          >
            <Pencil className="w-3.5 h-3.5" />
            Rename
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onArchive?.(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
          >
            <Archive className="w-3.5 h-3.5" />
            Archive
          </DropdownMenuItem>
          <DropdownMenuSeparator style={{ background: "hsla(220 10% 100% / 0.06)" }} />
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onDelete?.(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
            style={{ color: "hsl(0 70% 60%)" }}
          >
            <Trash2 className="w-3.5 h-3.5" />
            Delete
          </DropdownMenuItem>
        </>
      );

    case "in-progress":
    case "accepted":
    case "done":
      return (
        <>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onStartRename(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
          >
            <Pencil className="w-3.5 h-3.5" />
            Rename
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onReopen?.(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            Reopen
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onResetReaccept?.(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Reset & Re-accept
          </DropdownMenuItem>
          <DropdownMenuSeparator style={{ background: "hsla(220 10% 100% / 0.06)" }} />
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onDelete?.(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
            style={{ color: "hsl(0 70% 60%)" }}
          >
            <Trash2 className="w-3.5 h-3.5" />
            Delete
          </DropdownMenuItem>
        </>
      );

    case "archived":
      return (
        <>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onStartRename(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
          >
            <Pencil className="w-3.5 h-3.5" />
            Rename
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onReopen?.(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            Reopen
          </DropdownMenuItem>
          <DropdownMenuSeparator style={{ background: "hsla(220 10% 100% / 0.06)" }} />
          <DropdownMenuItem
            onClick={(e) => { e.stopPropagation(); onDelete?.(); }}
            className="text-[13px] cursor-pointer gap-2.5 py-2"
            style={{ color: "hsl(0 70% 60%)" }}
          >
            <Trash2 className="w-3.5 h-3.5" />
            Delete
          </DropdownMenuItem>
        </>
      );
  }
}

// ============================================================================
// Component
// ============================================================================

const isMutedGroup = (group: SessionGroup) => group === "done" || group === "archived";

export function PlanItem({
  plan,
  isSelected,
  group,
  progress,
  isEditing,
  editingTitle,
  isMenuOpen,
  inputRef,
  onSelect,
  onStartRename,
  onCancelRename: _onCancelRename,
  onConfirmRename,
  onTitleChange,
  onKeyDown,
  onMenuOpenChange,
  onArchive,
  onDelete,
  onReopen,
  onResetReaccept,
}: PlanItemProps) {
  const muted = isMutedGroup(group);

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
      onClick={() => !isEditing && onSelect()}
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
          <MessageSquare
            className="w-3 h-3"
            style={{ color: isSelected ? "hsl(14 100% 60%)" : "hsl(220 10% 50%)" }}
          />
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          {isEditing ? (
            <Input
              ref={inputRef}
              value={editingTitle}
              onChange={(e) => onTitleChange(e.target.value)}
              onKeyDown={onKeyDown}
              onBlur={onConfirmRename}
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
              </div>
              <MetadataLine group={group} plan={plan} {...(progress != null && { progress })} />
            </>
          )}
        </div>

        {/* Context Menu */}
        {!isEditing && (
          <DropdownMenu onOpenChange={onMenuOpenChange}>
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
                onStartRename={onStartRename}
                {...(onArchive != null && { onArchive })}
                {...(onDelete != null && { onDelete })}
                {...(onReopen != null && { onReopen })}
                {...(onResetReaccept != null && { onResetReaccept })}
              />
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
    </div>
  );
}
