/**
 * useIdeationHandlers - Event handlers for IdeationView
 */

import { useCallback, useRef, useState } from "react";
import type { TaskProposal, IdeationSession } from "@/types/ideation";
import type { ProactiveSyncNotification } from "@/stores/ideationStore";

export function useIdeationHandlers(
  session: IdeationSession | null,
  proposals: TaskProposal[],
  onSelectProposal: (proposalId: string) => void,
  onRemoveProposal: (proposalId: string) => void,
  onReorderProposals: (proposalIds: string[]) => void,
  onArchiveSession: (sessionId: string) => void,
  fetchPlanArtifact: (artifactId: string) => Promise<void>,
  dismissSyncNotification: () => void,
  syncNotification: ProactiveSyncNotification | null
) {
  const [highlightedProposalIds, setHighlightedProposalIds] = useState<Set<string>>(new Set());
  const [planHistoryDialog, setPlanHistoryDialog] = useState<{ isOpen: boolean; artifactId: string; version: number } | null>(null);
  const [isPlanExpanded, setIsPlanExpanded] = useState(false);
  const [importStatus, setImportStatus] = useState<{ type: "success" | "error"; message: string } | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const collapsePlan = useCallback(() => {
    setIsPlanExpanded(false);
  }, []);

  const handleArchive = useCallback(() => {
    if (session) onArchiveSession(session.id);
  }, [session, onArchiveSession]);

  const handleSelectAll = useCallback(() => {
    collapsePlan();
    proposals.forEach((p) => { if (!p.selected) onSelectProposal(p.id); });
  }, [proposals, onSelectProposal, collapsePlan]);

  const handleDeselectAll = useCallback(() => {
    collapsePlan();
    proposals.forEach((p) => { if (p.selected) onSelectProposal(p.id); });
  }, [proposals, onSelectProposal, collapsePlan]);

  const handleSortByPriority = useCallback(() => {
    collapsePlan();
    const sorted = [...proposals].sort((a, b) => b.priorityScore - a.priorityScore);
    onReorderProposals(sorted.map((p) => p.id));
  }, [proposals, onReorderProposals, collapsePlan]);

  const handleSelectProposal = useCallback((proposalId: string) => {
    collapsePlan();
    onSelectProposal(proposalId);
  }, [onSelectProposal, collapsePlan]);

  const handleClearAll = useCallback(() => {
    proposals.forEach((p) => onRemoveProposal(p.id));
  }, [proposals, onRemoveProposal]);

  const handleViewHistoricalPlan = useCallback((artifactId: string, version: number) => {
    setPlanHistoryDialog({ isOpen: true, artifactId, version });
  }, []);

  const handleClosePlanHistoryDialog = useCallback(() => setPlanHistoryDialog(null), []);

  const handleReviewSync = useCallback(() => {
    if (syncNotification) {
      setHighlightedProposalIds(new Set(syncNotification.proposalIds));
      setTimeout(() => setHighlightedProposalIds(new Set()), 5000);
    }
  }, [syncNotification]);

  const handleUndoSync = useCallback(() => {
    if (!syncNotification) return;
    dismissSyncNotification();
    setHighlightedProposalIds(new Set());
  }, [syncNotification, dismissSyncNotification]);

  const handleDismissSync = useCallback(() => {
    dismissSyncNotification();
    setHighlightedProposalIds(new Set());
  }, [dismissSyncNotification]);

  const handleImportPlan = useCallback(() => fileInputRef.current?.click(), []);

  const handleFileSelected = useCallback(async (event: React.ChangeEvent<HTMLInputElement>) => {
    if (!session) return;
    const file = event.target.files?.[0];
    if (!file) return;

    try {
      const content = await file.text();
      const title = file.name.replace(/\.md$/, "").replace(/_/g, " ");

      const apiResponse = await fetch("http://localhost:3847/api/create_plan_artifact", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ session_id: session.id, title, content }),
      });

      if (!apiResponse.ok) throw new Error("Failed to import plan");

      const data = await apiResponse.json();
      if (data.id) {
        await fetchPlanArtifact(data.id);
        setImportStatus({ type: "success", message: `Plan "${title}" imported successfully` });
        setTimeout(() => setImportStatus(null), 5000);
      }
    } catch (error) {
      console.error("Plan import error:", error);
      setImportStatus({ type: "error", message: error instanceof Error ? error.message : "Failed to import plan" });
      setTimeout(() => setImportStatus(null), 5000);
    } finally {
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  }, [session, fetchPlanArtifact]);

  return {
    highlightedProposalIds,
    planHistoryDialog,
    isPlanExpanded,
    setIsPlanExpanded,
    importStatus,
    setImportStatus,
    fileInputRef,
    handleArchive,
    handleSelectAll,
    handleDeselectAll,
    handleSortByPriority,
    handleSelectProposal,
    handleClearAll,
    handleViewHistoricalPlan,
    handleClosePlanHistoryDialog,
    handleReviewSync,
    handleUndoSync,
    handleDismissSync,
    handleImportPlan,
    handleFileSelected,
  };
}
