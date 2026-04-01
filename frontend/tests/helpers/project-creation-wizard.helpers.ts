import { Page } from "@playwright/test";

/**
 * Trigger the ProjectCreationWizard modal in web mode
 *
 * Opens the wizard by clicking the "New Project" option in the ProjectSelector dropdown.
 */
export async function openProjectCreationWizard(page: Page): Promise<void> {
  // Click the project selector trigger to open the dropdown
  await page.click('[data-testid="project-selector-trigger"]');

  // Wait for dropdown to appear
  await page.waitForSelector('[data-testid="new-project-option"]', { state: "visible" });

  // Click the "New Project" option
  await page.click('[data-testid="new-project-option"]');

  // Wait a small amount of time for modal to open
  await page.waitForTimeout(200);
}

/**
 * Create test project data
 */
export function createTestProjectData() {
  return {
    localMode: {
      name: "Test Project Local",
      workingDirectory: "/Users/test/projects/test-project",
      gitMode: "local" as const,
    },
    worktreeMode: {
      name: "Test Project Worktree",
      workingDirectory: "/Users/test/projects/test-project",
      gitMode: "worktree" as const,
      baseBranch: "main",
      worktreeBranch: "ralphx/test-feature",
      worktreePath: "/Users/test/projects/test-project/.ralphx-worktree",
    },
  };
}
