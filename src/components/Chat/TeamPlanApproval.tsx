/**
 * TeamPlanApproval — Inline approval banner for team plans
 *
 * Shows when a team lead requests plan approval via request_team_plan.
 * Displays teammate composition and approve/reject buttons.
 */

import React, { useCallback, useState } from "react";
import { Check, X, Users } from "lucide-react";
import { Button } from "@/components/ui/button";
import { approveTeamPlan, rejectTeamPlan } from "@/api/team";
import { useTeamStore } from "@/stores/teamStore";
import type { PendingTeamPlan } from "@/stores/teamStore";
import type { ContextType } from "@/types/chat-conversation";

interface TeamPlanApprovalProps {
  plan: PendingTeamPlan;
  contextType: ContextType;
  contextId: string;
}

export const TeamPlanApproval = React.memo(function TeamPlanApproval({
  plan,
  contextType,
  contextId,
}: TeamPlanApprovalProps) {
  const setPendingPlan = useTeamStore((s) => s.setPendingPlan);
  const [isApproving, setIsApproving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleApprove = useCallback(async () => {
    setIsApproving(true);
    setError(null);
    try {
      await approveTeamPlan(plan.planId, contextType, contextId);
      setPendingPlan(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to approve team plan");
      setIsApproving(false);
    }
  }, [plan.planId, contextType, contextId, setPendingPlan]);

  const handleReject = useCallback(async () => {
    try {
      await rejectTeamPlan(plan.planId);
    } catch {
      // Best-effort — clear UI regardless
    }
    setPendingPlan(null);
  }, [plan.planId, setPendingPlan]);

  return (
    <div
      className="mx-3 my-2 rounded-lg overflow-hidden"
      style={{
        backgroundColor: "hsl(220 10% 10%)",
        border: "1px solid hsl(25 80% 45% / 0.4)",
      }}
    >
      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-2"
        style={{
          backgroundColor: "hsl(25 80% 45% / 0.08)",
          borderBottom: "1px solid hsl(220 10% 14%)",
        }}
      >
        <Users className="w-3.5 h-3.5" style={{ color: "hsl(25 80% 55%)" }} />
        <span className="text-[11px] font-medium" style={{ color: "hsl(25 80% 65%)" }}>
          Team Plan — {plan.process}
        </span>
        <span
          className="ml-auto text-[10px] px-1.5 rounded"
          style={{
            backgroundColor: "hsl(220 10% 14%)",
            color: "hsl(220 10% 55%)",
          }}
        >
          {plan.teammates.length} teammate{plan.teammates.length !== 1 ? "s" : ""}
        </span>
      </div>

      {/* Teammate list */}
      <div className="px-3 py-2 space-y-1.5">
        {plan.teammates.map((mate, i) => (
          <div
            key={`${mate.role}-${i}`}
            className="flex items-center gap-2"
          >
            <span
              className="w-1.5 h-1.5 rounded-full shrink-0"
              style={{ backgroundColor: "hsl(220 10% 40%)" }}
            />
            <span className="text-[11px] font-medium" style={{ color: "hsl(220 10% 80%)" }}>
              {mate.role}
            </span>
            <span
              className="text-[10px] px-1 rounded"
              style={{
                backgroundColor: "hsl(220 10% 14%)",
                color: "hsl(220 10% 50%)",
              }}
            >
              {mate.model}
            </span>
            {mate.prompt_summary && (
              <span
                className="text-[10px] truncate flex-1"
                style={{ color: "hsl(220 10% 40%)" }}
              >
                {mate.prompt_summary}
              </span>
            )}
          </div>
        ))}
      </div>

      {/* Error */}
      {error && (
        <div className="px-3 pb-1">
          <span className="text-[10px]" style={{ color: "hsl(0 84% 60%)" }}>
            {error}
          </span>
        </div>
      )}

      {/* Actions */}
      <div
        className="flex items-center justify-end gap-2 px-3 py-2"
        style={{ borderTop: "1px solid hsl(220 10% 14%)" }}
      >
        <Button
          variant="ghost"
          size="sm"
          onClick={handleReject}
          disabled={isApproving}
          className="text-[11px] h-7 gap-1"
        >
          <X className="w-3 h-3" />
          Reject
        </Button>
        <Button
          size="sm"
          onClick={handleApprove}
          disabled={isApproving}
          className="text-[11px] h-7 gap-1"
          style={{
            backgroundColor: "hsl(25 80% 45%)",
            color: "white",
          }}
        >
          <Check className="w-3 h-3" />
          {isApproving ? "Spawning..." : "Approve & Spawn"}
        </Button>
      </div>
    </div>
  );
});
