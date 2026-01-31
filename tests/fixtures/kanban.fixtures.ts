import { Page } from "@playwright/test";
import { setupApp } from "./setup.fixtures";

/**
 * Setup kanban board with tasks in ALL workflow columns.
 *
 * The default workflow has 5 columns:
 * - Backlog (backlog)
 * - Ready (ready)
 * - Executing (executing)
 * - Review (pending_review)
 * - Approved (approved)
 *
 * This fixture creates tasks in each column to test full board rendering.
 */
export async function setupAllStatusColumns(page: Page) {
  await setupApp(page);

  // Inject tasks for all 5 workflow columns
  await page.evaluate(() => {
    const mockStore = (window as any).__mockStore;
    const queryClient = (window as any).__queryClient;

    if (!mockStore) {
      throw new Error("Mock store not available");
    }

    // Get the default project ID from existing data
    const projectId = Array.from(mockStore.projects.keys())[0];

    // Statuses matching the 5 default workflow columns
    const statusesPerColumn = [
      { status: "backlog", title: "Design new feature" },
      { status: "ready", title: "Implement API endpoint" },
      { status: "executing", title: "Build dashboard UI" },
      { status: "pending_review", title: "Add authentication" },
      { status: "approved", title: "Initial project setup" },
    ];

    // Clear existing tasks
    mockStore.tasks.clear();

    // Create multiple tasks per column for visual density
    const now = new Date().toISOString();
    statusesPerColumn.forEach(({ status, title }, colIndex) => {
      // Create 2-3 tasks per column
      for (let i = 0; i < 3; i++) {
        const taskId = `task-${status}-${i}`;
        const task = {
          id: taskId,
          projectId,
          category: "feature",
          title: `${title} ${i + 1}`,
          description: `Task ${i + 1} in ${status} column`,
          priority: colIndex * 10 + i,
          internalStatus: status,
          needsReviewPoint: false,
          createdAt: now,
          updatedAt: now,
          startedAt: status === "executing" ? now : null,
          completedAt: status === "approved" ? now : null,
          archivedAt: null,
          blockedReason: null,
        };
        mockStore.tasks.set(taskId, task);
      }
    });

    // Invalidate cache to trigger refetch
    if (queryClient) {
      queryClient.invalidateQueries();
    }
  });

  // Wait for queries to refetch and render
  await page.waitForTimeout(500);
  await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });
}
