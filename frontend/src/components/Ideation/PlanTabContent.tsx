import { useState, useCallback, useEffect, useRef } from "react";
import { Upload } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { useIdeationStore } from "@/stores/ideationStore";
import { PlanDisplay } from "./PlanDisplay";
import type { TeamMetadata } from "./PlanDisplay";
import { PlanEditor } from "./PlanEditor";
import { AcceptedSessionBanner } from "./AcceptedSessionBanner";
import { PendingAcceptanceBanner } from "./PendingAcceptanceBanner";
import { PlanEmptyState } from "./PlanEmptyState";
import { ExportPlanDialog } from "./ExportPlanDialog";
import { chatApi } from "@/api/chat";
import type { IdeationSession, TaskProposal } from "@/types/ideation";

// ============================================================================
// Types
// ============================================================================

interface PlanTabContentProps {
  session: IdeationSession;
  proposals: TaskProposal[];
  teamMetadata?: TeamMetadata;
  importStatus: { type: "success" | "error"; message: string } | null;
  onImportStatusChange: (status: { type: "success" | "error"; message: string } | null) => void;
  onImportPlan: () => void;
  onViewWork: () => void;
  /** isPlanExpanded + handler managed by parent (useIdeationHandlers) for test compatibility */
  isPlanExpanded: boolean;
  onExpandedChange: (expanded: boolean) => void;
  /** Historical plan version to display (set by parent when user clicks from proposal card) */
  requestedHistoricalVersion: number | null;
  onHistoricalVersionViewed: () => void;
}

// ============================================================================
// Component
// ============================================================================

export function PlanTabContent({
  session,
  proposals,
  teamMetadata,
  importStatus,
  onImportStatusChange,
  onImportPlan,
  onViewWork,
  isPlanExpanded,
  onExpandedChange,
  requestedHistoricalVersion,
  onHistoricalVersionViewed,
}: PlanTabContentProps) {
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const isEditingRef = useRef(false);

  // Keep ref in sync so the planArtifact effect can read latest value without stale closure
  useEffect(() => {
    isEditingRef.current = isEditing;
  }, [isEditing]);

  // Read from store — efficient (Zustand only re-renders on actual changes)
  const planArtifact = useIdeationStore((state) => state.planArtifact);
  const setPlanArtifact = useIdeationStore((state) => state.setPlanArtifact);

  // Reset editing mode when plan changes externally
  useEffect(() => {
    if (isEditingRef.current) {
      toast.info("Plan was updated externally. Exiting editor.");
    }
    setIsEditing(false);
  }, [planArtifact?.id, planArtifact?.metadata?.version]);

  const handleCreateProposals = useCallback(async () => {
    try {
      await chatApi.sendAgentMessage("ideation", session.id, "create task proposals from the approved plan");
    } catch (err) {
      console.error("Failed to create proposals:", err);
      toast.error("Failed to request proposal creation");
    }
  }, [session.id]);

  return (
    <div className="flex-1 overflow-y-auto p-4">
      {/* Accepted session banner */}
      {session.status === "accepted" && (
        <AcceptedSessionBanner
          projectId={session.projectId}
          proposals={proposals}
          convertedAt={session.convertedAt}
          onViewWork={onViewWork}
        />
      )}

      {/* Pending acceptance banner — shown after agent-initiated finalization gate */}
      {session.acceptanceStatus === "pending" && (
        <PendingAcceptanceBanner sessionId={session.id} />
      )}

      {/* Import status notification */}
      {importStatus && (
        <div
          className="mb-4 p-4 rounded-xl"
          style={{
            background: importStatus.type === "success"
              ? "hsla(145 70% 40% / 0.1)"
              : "hsla(0 70% 50% / 0.1)",
            border: `1px solid ${importStatus.type === "success"
              ? "hsla(145 70% 40% / 0.3)"
              : "hsla(0 70% 50% / 0.3)"}`,
          }}
        >
          <div className="flex items-center justify-between">
            <p className="text-sm font-medium" style={{ color: "hsl(220 10% 90%)" }}>{importStatus.message}</p>
            <Button variant="ghost" size="icon" onClick={() => onImportStatusChange(null)} className="h-7 w-7">×</Button>
          </div>
        </div>
      )}

      {/* Import plan button — shown when no plan artifact but proposals exist */}
      {!planArtifact && proposals.length > 0 && (
        <Button
          variant="outline"
          onClick={onImportPlan}
          className="w-full mb-4 gap-2 transition-colors duration-150"
          style={{
            border: "1px solid hsla(220 10% 100% / 0.1)",
            background: "transparent",
            color: "hsl(220 10% 70%)",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.2)";
            e.currentTarget.style.background = "hsla(220 10% 100% / 0.03)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.borderColor = "hsla(220 10% 100% / 0.1)";
            e.currentTarget.style.background = "transparent";
          }}
          data-testid="import-plan-button"
        >
          <Upload className="w-4 h-4" />
          Import Implementation Plan
        </Button>
      )}

      {/* Plan display / editor */}
      {planArtifact && (
        <div className="mb-4">
          {isEditing ? (
            <PlanEditor
              plan={planArtifact}
              onSave={(updated) => {
                setPlanArtifact(updated);
                setIsEditing(false);
              }}
              onCancel={() => setIsEditing(false)}
            />
          ) : (
            <PlanDisplay
              plan={planArtifact}
              linkedProposalsCount={proposals.filter((p) => p.planArtifactId === planArtifact.id).length}
              onEdit={() => {
                onHistoricalVersionViewed?.();
                setIsEditing(true);
              }}
              onExport={() => setExportDialogOpen(true)}
              isExpanded={isPlanExpanded}
              onExpandedChange={onExpandedChange}
              {...(teamMetadata !== undefined && { teamMetadata })}
              {...(requestedHistoricalVersion !== null && {
                requestedVersion: requestedHistoricalVersion,
                onVersionViewed: onHistoricalVersionViewed,
              })}
              onCreateProposals={handleCreateProposals}
            />
          )}
        </div>
      )}

      <ExportPlanDialog
        open={exportDialogOpen}
        onOpenChange={setExportDialogOpen}
        sessionId={session.id}
        sessionTitle={session.title ?? null}
        verificationStatus={session.verificationStatus ?? "unverified"}
        planArtifact={planArtifact}
        projectId={session.projectId}
      />

      {/* Empty state — shown when no plan and no proposals */}
      {!planArtifact && proposals.length === 0 && (
        <PlanEmptyState onBrowse={onImportPlan} />
      )}
    </div>
  );
}
