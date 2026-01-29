/**
 * ProjectCreationWizard helpers - validation and utility functions
 */

import type { GitMode } from "@/types/project";

// ============================================================================
// Form State Interface
// ============================================================================

export interface FormState {
  name: string;
  workingDirectory: string;
  gitMode: GitMode;
  worktreeBranch: string;
  baseBranch: string;
}

export interface FormErrors {
  name?: string;
  workingDirectory?: string;
  worktreeBranch?: string;
  baseBranch?: string;
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Generate default branch name from project name
 * Format: ralphx/<project-name-slug>
 */
export function generateBranchName(projectName: string): string {
  const slug = projectName
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return slug ? `ralphx/${slug}` : "ralphx/feature";
}

/**
 * Generate worktree path from working directory and branch
 * Format: ~/ralphx-worktrees/<project-folder-name>
 */
export function generateWorktreePath(workingDirectory: string): string {
  const folderName = workingDirectory.split("/").pop() || "project";
  return `~/ralphx-worktrees/${folderName}`;
}

/**
 * Extract folder name from path for auto-inferring project name
 */
export function extractFolderName(path: string): string {
  const folderName = path.split("/").pop() || "";
  return folderName;
}

/**
 * Validate the form and return errors
 * Note: Project name is optional - will be inferred from folder if empty
 */
export function validateForm(form: FormState): FormErrors {
  const errors: FormErrors = {};

  // Project name is now optional - will be inferred from folder
  // Only validate if user explicitly set it to something invalid
  // (not just empty)

  if (!form.workingDirectory.trim()) {
    errors.workingDirectory = "Location is required";
  }

  if (form.gitMode === "worktree") {
    if (!form.worktreeBranch.trim()) {
      errors.worktreeBranch = "Branch name is required";
    } else if (!/^[a-zA-Z0-9/_-]+$/.test(form.worktreeBranch)) {
      errors.worktreeBranch = "Branch name contains invalid characters";
    }

    if (!form.baseBranch.trim()) {
      errors.baseBranch = "Base branch is required";
    }
  }

  return errors;
}
