/**
 * PlanBrowser - macOS Tahoe styled sidebar for ideation plans
 *
 * Design: Native macOS sidebar with frosted glass, refined typography,
 * and smooth spring animations. Warm orange accent (#ff6b35).
 */

import { useMemo, useState, useRef, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  MessageSquare,
  Plus,
  Clock,
  Sparkles,
  MoreHorizontal,
  Pencil,
  Archive,
  Trash2,
  History,
  ChevronDown,
  CheckCircle,
} from "lucide-react";
import { cn } from "@/lib/utils";
import type { IdeationSession } from "@/types/ideation";
import { ideationApi } from "@/api/ideation";

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

// ============================================================================
// Types
// ============================================================================

interface PlanBrowserProps {
  plans: IdeationSession[];
  historyPlans: IdeationSession[];
  currentPlanId: string | null;
  onSelectPlan: (planId: string) => void;
  onNewPlan: () => void;
  onArchivePlan?: (planId: string) => void;
  onDeletePlan?: (planId: string) => void;
}

// ============================================================================
// Plan Item Component
// ============================================================================

interface PlanItemProps {
  plan: IdeationSession;
  isSelected: boolean;
  isHistory: boolean;
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
}

function PlanItem({
  plan,
  isSelected,
  isHistory,
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
}: PlanItemProps) {
  const statusBadge = isHistory ? (
    <span
      className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[9px] font-medium"
      style={{
        background: plan.status === "accepted"
          ? "hsla(145 70% 40% / 0.15)"
          : "hsla(220 10% 100% / 0.08)",
        color: plan.status === "accepted"
          ? "hsl(145 70% 60%)"
          : "hsl(220 10% 60%)",
        border: plan.status === "accepted"
          ? "1px solid hsla(145 70% 40% / 0.3)"
          : "1px solid hsla(220 10% 100% / 0.1)",
      }}
    >
      {plan.status === "accepted" && <CheckCircle className="w-2.5 h-2.5" />}
      {plan.status === "accepted" ? "Accepted" : "Archived"}
    </span>
  ) : null;

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
                    color: isSelected ? "hsl(220 10% 90%)" : "hsl(220 10% 70%)",
                  }}
                >
                  {plan.title || "Untitled Plan"}
                </span>
                {statusBadge}
              </div>
              <div
                className="flex items-center gap-1 text-[10px]"
                style={{ color: "hsl(220 10% 45%)" }}
              >
                <Clock className="w-2.5 h-2.5" />
                <span>{formatRelativeTime(plan.updatedAt)}</span>
              </div>
            </>
          )}
        </div>

        {/* Menu - only show for non-history items (history items are read-only) */}
        {!isEditing && !isHistory && (
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
              className="w-40"
              style={{
                background: "hsl(220 10% 14%)",
                border: "1px solid hsla(220 10% 100% / 0.08)",
                boxShadow: "0 8px 32px hsla(0 0% 0% / 0.4)",
              }}
            >
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation();
                  onStartRename();
                }}
                className="text-[13px] cursor-pointer gap-2.5 py-2"
              >
                <Pencil className="w-3.5 h-3.5" />
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation();
                  onArchive?.();
                }}
                className="text-[13px] cursor-pointer gap-2.5 py-2"
              >
                <Archive className="w-3.5 h-3.5" />
                Archive
              </DropdownMenuItem>
              <DropdownMenuSeparator style={{ background: "hsla(220 10% 100% / 0.06)" }} />
              <DropdownMenuItem
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete?.();
                }}
                className="text-[13px] cursor-pointer gap-2.5 py-2"
                style={{ color: "hsl(0 70% 60%)" }}
              >
                <Trash2 className="w-3.5 h-3.5" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        )}
      </div>
    </div>
  );
}

// ============================================================================
// Component
// ============================================================================

export function PlanBrowser({
  plans,
  historyPlans,
  currentPlanId,
  onSelectPlan,
  onNewPlan,
  onArchivePlan,
  onDeletePlan,
}: PlanBrowserProps) {
  const sortedPlans = useMemo(
    () => [...plans].sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()),
    [plans]
  );

  const sortedHistoryPlans = useMemo(
    () => [...historyPlans].sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()),
    [historyPlans]
  );

  const [editingPlanId, setEditingPlanId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState("");
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const [isHistoryOpen, setIsHistoryOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editingPlanId && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editingPlanId]);

  const handleStartRename = (plan: IdeationSession) => {
    setEditingPlanId(plan.id);
    setEditingTitle(plan.title || "");
  };

  const handleCancelRename = () => {
    setEditingPlanId(null);
    setEditingTitle("");
  };

  const handleConfirmRename = async (planId: string) => {
    const trimmedTitle = editingTitle.trim();
    if (trimmedTitle) {
      try {
        await ideationApi.sessions.updateTitle(planId, trimmedTitle);
      } catch (error) {
        console.error("Failed to rename plan:", error);
      }
    }
    setEditingPlanId(null);
    setEditingTitle("");
  };

  const handleKeyDown = (e: React.KeyboardEvent, planId: string) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleConfirmRename(planId);
    } else if (e.key === "Escape") {
      e.preventDefault();
      handleCancelRename();
    }
  };

  const totalPlans = plans.length + historyPlans.length;

  return (
    <div
      data-testid="plan-browser"
      className="flex flex-col h-full"
      style={{
        width: "276px",
        minWidth: "276px",
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
                {totalPlans} {totalPlans === 1 ? "plan" : "plans"}
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
          {sortedPlans.length === 0 && sortedHistoryPlans.length === 0 ? (
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
              {/* Active Plans */}
              <div className="space-y-1">
                {sortedPlans.map((plan) => (
                  <PlanItem
                    key={plan.id}
                    plan={plan}
                    isSelected={plan.id === currentPlanId}
                    isHistory={false}
                    isEditing={editingPlanId === plan.id}
                    editingTitle={editingTitle}
                    isMenuOpen={openMenuId === plan.id}
                    inputRef={inputRef}
                    onSelect={() => onSelectPlan(plan.id)}
                    onStartRename={() => handleStartRename(plan)}
                    onCancelRename={handleCancelRename}
                    onConfirmRename={() => handleConfirmRename(plan.id)}
                    onTitleChange={setEditingTitle}
                    onKeyDown={(e) => handleKeyDown(e, plan.id)}
                    onMenuOpenChange={(open) => setOpenMenuId(open ? plan.id : null)}
                    onArchive={() => onArchivePlan?.(plan.id)}
                    onDelete={() => onDeletePlan?.(plan.id)}
                  />
                ))}
              </div>

              {/* History Section (Collapsible) */}
              {sortedHistoryPlans.length > 0 && (
                <Collapsible
                  open={isHistoryOpen}
                  onOpenChange={setIsHistoryOpen}
                  className="mt-3"
                >
                  <CollapsibleTrigger asChild>
                    <button
                      className="w-full flex items-center gap-2 px-2 py-1.5 rounded-md transition-colors duration-150"
                      style={{
                        color: "hsl(220 10% 50%)",
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.background = "hsla(220 10% 100% / 0.04)";
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.background = "transparent";
                      }}
                    >
                      <History className="w-3.5 h-3.5" />
                      <span className="text-[11px] font-medium tracking-[-0.01em]">
                        History ({sortedHistoryPlans.length})
                      </span>
                      <ChevronDown
                        className={cn(
                          "w-3 h-3 ml-auto transition-transform duration-200",
                          isHistoryOpen && "rotate-180"
                        )}
                      />
                    </button>
                  </CollapsibleTrigger>
                  <CollapsibleContent className="mt-1 space-y-1">
                    {sortedHistoryPlans.map((plan) => (
                      <PlanItem
                        key={plan.id}
                        plan={plan}
                        isSelected={plan.id === currentPlanId}
                        isHistory={true}
                        isEditing={editingPlanId === plan.id}
                        editingTitle={editingTitle}
                        isMenuOpen={openMenuId === plan.id}
                        inputRef={inputRef}
                        onSelect={() => onSelectPlan(plan.id)}
                        onStartRename={() => handleStartRename(plan)}
                        onCancelRename={handleCancelRename}
                        onConfirmRename={() => handleConfirmRename(plan.id)}
                        onTitleChange={setEditingTitle}
                        onKeyDown={(e) => handleKeyDown(e, plan.id)}
                        onMenuOpenChange={(open) => setOpenMenuId(open ? plan.id : null)}
                      />
                    ))}
                  </CollapsibleContent>
                </Collapsible>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
