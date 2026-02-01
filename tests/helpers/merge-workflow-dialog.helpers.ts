/**
 * Test helpers for MergeWorkflowDialog component
 */

import type { Page } from "@playwright/test";

export interface MockMergeWorkflowState {
  isOpen: boolean;
  project: {
    id: string;
    name: string;
    path: string;
    worktree_path: string | null;
    status: string;
    created_at: string;
    updated_at: string;
  };
  completionData: {
    commitCount: number;
    branchName: string;
  };
  isProcessing?: boolean;
  error?: string | null;
  showViewDiff?: boolean;
  showViewCommits?: boolean;
}

/**
 * Open the MergeWorkflowDialog with test data
 */
export async function openMergeWorkflowDialog(
  page: Page,
  state?: Partial<MockMergeWorkflowState>
): Promise<void> {
  const defaultState: MockMergeWorkflowState = {
    isOpen: true,
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
  };

  const mergedState = { ...defaultState, ...state };

  await page.evaluate((mockState) => {
    // Create a global store for the dialog state
    (window as any).__mergeWorkflowDialogState = mockState;

    // Dispatch a custom event to trigger the dialog
    window.dispatchEvent(
      new CustomEvent("openMergeWorkflowDialog", {
        detail: mockState,
      })
    );
  }, mergedState);

  // Wait for the dialog to appear
  await page.waitForSelector('[data-testid="merge-workflow-dialog"]', {
    timeout: 5000,
  });
}

/**
 * Close the MergeWorkflowDialog
 */
export async function closeMergeWorkflowDialog(page: Page): Promise<void> {
  await page.evaluate(() => {
    if ((window as any).__mergeWorkflowDialogState) {
      (window as any).__mergeWorkflowDialogState.isOpen = false;
    }
  });

  // Wait for the dialog to disappear
  await page.waitForSelector('[data-testid="merge-workflow-dialog"]', {
    state: "hidden",
    timeout: 5000,
  });
}

/**
 * Set the dialog to processing state
 */
export async function setMergeWorkflowProcessing(
  page: Page,
  isProcessing: boolean
): Promise<void> {
  await page.evaluate((processing) => {
    if ((window as any).__mergeWorkflowDialogState) {
      (window as any).__mergeWorkflowDialogState.isProcessing = processing;
    }
    window.dispatchEvent(new CustomEvent("updateMergeWorkflowDialog"));
  }, isProcessing);
}

/**
 * Set the dialog error state
 */
export async function setMergeWorkflowError(
  page: Page,
  error: string | null
): Promise<void> {
  await page.evaluate((errorMsg) => {
    if ((window as any).__mergeWorkflowDialogState) {
      (window as any).__mergeWorkflowDialogState.error = errorMsg;
    }
    window.dispatchEvent(new CustomEvent("updateMergeWorkflowDialog"));
  }, error);
}
