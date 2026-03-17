import { useState, useCallback } from "react";
import { Loader2, Upload } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { useIdeationStore } from "@/stores/ideationStore";
import { PlanDisplay } from "./PlanDisplay";
import type { TeamMetadata } from "./PlanDisplay";
import { AcceptedSessionBanner } from "./AcceptedSessionBanner";
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
  isReadOnly: boolean;
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
  isReadOnly: _isReadOnly,
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

  // Read from store — efficient (Zustand only re-renders on actual changes)
  const planArtifact = useIdeationStore((state) => state.planArtifact);
  const ideationSettings = useIdeationStore((state) => state.ideationSettings);

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

      {/* Plan display */}
      {planArtifact && (
        <div className="mb-4">
          <PlanDisplay
            plan={planArtifact}
            showApprove={ideationSettings?.requirePlanApproval ?? false}
            linkedProposalsCount={proposals.filter((p) => p.planArtifactId === planArtifact.id).length}
            onEdit={() => {}}
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

      {/* Waiting for plan — shown when plan is required but not yet created */}
      {!planArtifact && ideationSettings?.planMode === "required" && proposals.length === 0 && (
        <div className="flex flex-col items-center justify-center h-full p-8">
          <div className="relative">
            <div
              className="relative p-8 rounded-2xl text-center"
              style={{
                background: "hsla(220 10% 14% / 0.6)",
                border: "1px solid hsla(220 10% 100% / 0.06)",
              }}
            >
              <Loader2 className="w-10 h-10 mx-auto mb-4 animate-spin" style={{ color: "hsl(14 100% 60%)" }} />
              <p className="font-medium" style={{ color: "hsl(220 10% 70%)" }}>Waiting for implementation plan...</p>
              <p className="text-sm mt-1" style={{ color: "hsl(220 10% 50%)" }}>The orchestrator will create a plan first</p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
