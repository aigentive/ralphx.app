/**
 * PlanBrowser - macOS Tahoe styled sidebar for ideation plans
 *
 * Design: Native macOS sidebar with frosted glass, refined typography,
 * and smooth spring animations. Warm orange accent (#ff6b35).
 *
 * Five semantic groups: Drafts, In Progress, Accepted, Done, Archived.
 */

import { useMemo, useState, useRef, useEffect } from "react";
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
import type { IdeationSession } from "@/types/ideation";
import { ideationApi } from "@/api/ideation";
import { PlanItem } from "./PlanItem";
import { SessionGroupHeader } from "./SessionGroupHeader";
import { groupSessions, type SessionGroup } from "./planBrowserUtils";
import { useSessionProgress } from "@/hooks/useSessionProgress";

// ============================================================================
// Types
// ============================================================================

interface PlanBrowserProps {
  sessions: IdeationSession[];
  projectId: string;
  currentPlanId: string | null;
  onSelectPlan: (planId: string) => void;
  onNewPlan: () => void;
  onArchivePlan?: (planId: string) => void;
  onDeletePlan?: (planId: string) => void;
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
  defaultOpen: boolean;
}[] = [
  { key: "drafts", label: "Drafts", icon: Pencil, defaultOpen: true },
  { key: "in-progress", label: "In Progress", icon: Zap, accentColor: "hsl(14 100% 60%)", defaultOpen: true },
  { key: "accepted", label: "Accepted", icon: CheckCircle, accentColor: "hsl(145 70% 45%)", defaultOpen: true },
  { key: "done", label: "Done", icon: CircleCheck, accentColor: "hsl(220 10% 45%)", defaultOpen: false },
  { key: "archived", label: "Archived", icon: Archive, accentColor: "hsl(220 10% 45%)", defaultOpen: false },
];

// ============================================================================
// Component
// ============================================================================

export function PlanBrowser({
  sessions,
  projectId,
  currentPlanId,
  onSelectPlan,
  onNewPlan,
  onArchivePlan,
  onDeletePlan,
  onReopenPlan,
  onResetReacceptPlan,
}: PlanBrowserProps) {
  const { progressMap } = useSessionProgress(projectId, sessions);
  const grouped = useMemo(() => groupSessions(sessions, progressMap), [sessions, progressMap]);

  const [editingPlanId, setEditingPlanId] = useState<string | null>(null);
  const [editingTitle, setEditingTitle] = useState("");
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const [groupOpen, setGroupOpen] = useState<Record<SessionGroup, boolean>>({
    drafts: true,
    "in-progress": true,
    accepted: true,
    done: false,
    archived: false,
  });
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

  const handleGroupToggle = (group: SessionGroup, open: boolean) => {
    setGroupOpen((prev: Record<SessionGroup, boolean>) => ({ ...prev, [group]: open }));
  };

  const renderPlanItem = (plan: IdeationSession, group: SessionGroup) => {
    const progress = progressMap.get(plan.id);
    return (
      <PlanItem
        key={plan.id}
        plan={plan}
        isSelected={plan.id === currentPlanId}
        group={group}
        {...(progress != null && { progress })}
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
        onReopen={() => onReopenPlan?.(plan.id)}
        onResetReaccept={() => onResetReacceptPlan?.(plan.id)}
      />
    );
  };

  const hasAnySessions = sessions.length > 0;

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
                {sessions.length} {sessions.length === 1 ? "plan" : "plans"}
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
                const items = grouped[key];
                if (items.length === 0) return null;

                // Drafts group renders flat (always expanded, no collapsible header)
                if (key === "drafts") {
                  return (
                    <div key={key} className="space-y-1">
                      {items.map((plan) => renderPlanItem(plan, key))}
                    </div>
                  );
                }

                return (
                  <SessionGroupHeader
                    key={key}
                    icon={icon}
                    label={label}
                    count={items.length}
                    isOpen={groupOpen[key]}
                    onToggle={(open) => handleGroupToggle(key, open)}
                    {...(accentColor != null && { accentColor })}
                  >
                    {items.map((plan) => renderPlanItem(plan, key))}
                  </SessionGroupHeader>
                );
              })}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
