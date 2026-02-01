/**
 * Test page for MergeWorkflowDialog visual testing
 * This component is ONLY used in Playwright tests
 */

import { useState, useEffect } from "react";
import { MergeWorkflowDialog, type MergeWorkflowDialogProps } from "./MergeWorkflowDialog";

export function MergeWorkflowDialogTestPage() {
  const [dialogState, setDialogState] = useState<Partial<MergeWorkflowDialogProps> & { showViewDiff?: boolean; showViewCommits?: boolean }>({
    isOpen: false,
    project: {
      id: "test-project-1",
      name: "Test Project",
      path: "/path/to/project",
      worktree_path: null,
      status: "active",
      created_at: "2026-01-31T10:00:00+00:00",
      updated_at: "2026-01-31T10:00:00+00:00",
    },
    completionData: {
      commitCount: 5,
      branchName: "ralphx/test-feature",
    },
    isProcessing: false,
    error: null,
    showViewDiff: false,
    showViewCommits: false,
    onClose: () => setDialogState((s) => ({ ...s, isOpen: false })),
    onConfirm: (result) => {
      console.log("Confirmed:", result);
      setDialogState((s) => ({ ...s, isOpen: false }));
    },
  });

  useEffect(() => {
    // Listen for test events to open/update the dialog
    const handleOpen = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      setDialogState((s) => ({ ...s, ...detail, isOpen: true }));
    };

    const handleUpdate = () => {
      const state = (window as any).__mergeWorkflowDialogState;
      if (state) {
        setDialogState((s) => ({ ...s, ...state }));
      }
    };

    window.addEventListener("openMergeWorkflowDialog", handleOpen);
    window.addEventListener("updateMergeWorkflowDialog", handleUpdate);

    return () => {
      window.removeEventListener("openMergeWorkflowDialog", handleOpen);
      window.removeEventListener("updateMergeWorkflowDialog", handleUpdate);
    };
  }, []);

  if (!dialogState.isOpen) {
    return (
      <div className="p-8 text-center text-[var(--text-muted)]">
        <p>MergeWorkflowDialog test page</p>
        <p className="text-xs mt-2">Use test helpers to open the dialog</p>
      </div>
    );
  }

  return (
    <MergeWorkflowDialog
      isOpen={dialogState.isOpen || false}
      onClose={dialogState.onClose || (() => {})}
      onConfirm={dialogState.onConfirm || (() => {})}
      project={dialogState.project!}
      completionData={dialogState.completionData!}
      onViewDiff={dialogState.showViewDiff ? () => console.log("View diff") : undefined}
      onViewCommits={dialogState.showViewCommits ? () => console.log("View commits") : undefined}
      isProcessing={dialogState.isProcessing}
      error={dialogState.error}
    />
  );
}
