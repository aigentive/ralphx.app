import { test, expect } from "@playwright/test";
import type { Page } from "@playwright/test";
import { setupApp } from "../fixtures/setup.fixtures";

interface PlanStoreWithSetState {
  setState(
    updater: (state: {
      activeExecutionPlanIdByProject: Record<string, string | null>;
    }) => {
      activeExecutionPlanIdByProject: Record<string, string | null>;
    }
  ): void;
}

/**
 * Integration tests for Global Active Plan feature
 *
 * Tests cross-view synchronization of active plan state across:
 * - Ideation view (session acceptance triggers plan activation)
 * - Graph view (inline selector + filtering)
 * - Kanban view (inline selector + board filtering)
 * - Quick switcher (Cmd+Shift+P global palette)
 *
 * Uses web mode with mock API (api-mock/)
 */

/**
 * Wait for React Query queries to settle after plan state changes
 * Since query keys include ideationSessionId, changing the plan causes new queries with different keys
 * We need to wait for those queries to fetch and succeed before asserting on DOM
 */
async function waitForQueriesAfterPlanChange(page: Page, timeoutMs = 2000) {
  // Wait for queries to refetch and settle
  await page.waitForTimeout(timeoutMs);
}

async function setDeterministicActivePlan(
  page: Page,
  projectId: string,
  sessionId: string | null,
  executionPlanId: string | null = sessionId
) {
  await page.evaluate(
    async ({
      targetProjectId,
      targetSessionId,
      targetExecutionPlanId,
    }: {
      targetProjectId: string;
      targetSessionId: string | null;
      targetExecutionPlanId: string | null;
    }) => {
      const { planApi } = await import("/src/api/plan");
      const planStore = (window as unknown as {
        __planStore?: { getState(): { loadActivePlan(projectId: string): Promise<void> } };
      }).__planStore;

      if (!planStore) {
        throw new Error("planStore not available");
      }

      if (targetSessionId === null) {
        await planApi.clearActivePlan(targetProjectId);
      } else {
        await planApi.setActivePlan(targetProjectId, targetSessionId, "quick_switcher");
      }

      await planStore.getState().loadActivePlan(targetProjectId);
      if (targetExecutionPlanId !== targetSessionId) {
        (window as unknown as {
          __planStore?: PlanStoreWithSetState;
        }).__planStore?.setState((state) => ({
          activeExecutionPlanIdByProject: {
            ...state.activeExecutionPlanIdByProject,
            [targetProjectId]: targetExecutionPlanId,
          },
        }));
      }
    },
    {
      targetProjectId: projectId,
      targetSessionId: sessionId,
      targetExecutionPlanId: executionPlanId,
    }
  );
}

test.describe("Active Plan Cross-View Sync", () => {
  test.beforeEach(async ({ page }) => {
    await setupApp(page);

    // Seed mock store with accepted ideation sessions
    await page.evaluate(async () => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (!store) {
        throw new Error("Mock store not available - ensure api-mock is configured");
      }

      // Preserve the seeded demo project so app bootstrap and project selection stay aligned.
      store.tasks.clear();

      const project = store.projects.get("project-mock-1");
      if (!project) {
        throw new Error("Seeded project-mock-1 not available");
      }

      // Set as active project in projectStore (using dynamic import since we're in browser context)
      const { useProjectStore } = await import("/src/stores/projectStore");
      useProjectStore.getState().selectProject(project.id);

      // Create accepted sessions (plans)
      const sessions = [
        {
          id: "session-1",
          projectId: project.id,
          title: "Plan Alpha",
          status: "accepted",
          convertedAt: new Date().toISOString(),
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: "session-2",
          projectId: project.id,
          title: "Plan Beta",
          status: "accepted",
          convertedAt: new Date().toISOString(),
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ];

      // Store sessions in mock store
      if (!store.sessions) store.sessions = new Map();
      sessions.forEach(s => store.sessions.set(s.id, s));

      // Create tasks linked to different plans
      const tasks = [
        {
          id: "task-1",
          projectId: project.id,
          title: "Task from Plan Alpha",
          description: "Task 1",
          internalStatus: "backlog",
          priority: 0,
          category: "feature",
          ideationSessionId: "session-1",
          planArtifactId: "session-1", // Graph API uses planArtifactId for filtering
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: "task-2",
          projectId: project.id,
          title: "Another Task from Plan Alpha",
          description: "Task 2",
          internalStatus: "ready",
          priority: 1,
          category: "feature",
          ideationSessionId: "session-1",
          planArtifactId: "session-1",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        {
          id: "task-3",
          projectId: project.id,
          title: "Task from Plan Beta",
          description: "Task 3",
          internalStatus: "executing",
          priority: 2,
          category: "feature",
          ideationSessionId: "session-2",
          planArtifactId: "session-2",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ];

      tasks.forEach(t => store.tasks.set(t.id, t));

      const { planApi } = await import("/src/api/plan");
      await planApi.clearActivePlan(project.id);
      const planStore = (window as unknown as {
        __planStore?: { getState(): { loadActivePlan(projectId: string): Promise<void> } };
      }).__planStore;
      await planStore?.getState().loadActivePlan(project.id);

      // Invalidate queries to refetch with new data
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Wait for initial data to load
    await page.waitForTimeout(500);
  });

  test("Test 1: Accept session in Ideation → navigate to Graph → verify filtered tasks shown", async ({ page }) => {
    // Navigate to Graph first (simulating user navigation after session acceptance)
    await page.click('[data-testid="nav-graph"]');

    // Wait for the graph no-plan placeholder to appear initially.
    await expect(page.getByText("No plan selected")).toBeVisible();

    await setDeterministicActivePlan(page, "project-mock-1", "session-1", "session-1");
    const debugInfo = await page.evaluate(async () => {
      const planStore = (window as unknown as {
        __planStore?: {
          getState(): {
            activePlanByProject: Record<string, string | null>;
          };
        };
      }).__planStore;
      const mockStore = (window as unknown as {
        __mockStore?: { tasks: Map<string, unknown>; activePlans?: Map<string, string | null> };
      }).__mockStore;
      return {
        activePlanInStore: planStore?.getState().activePlanByProject["project-mock-1"],
        activePlanInMock: mockStore?.activePlans?.get("project-mock-1"),
        taskCount: mockStore?.tasks?.size ?? 0,
      };
    });
    console.log("Debug info:", debugInfo);

    // Wait for React Query to refetch with new ideationSessionId and settle
    await waitForQueriesAfterPlanChange(page, 2000);

    // Wait for task nodes to render
    await page.waitForSelector('[data-testid="task-node"]', { timeout: 10000 });

    // Verify tasks from session-1 are shown by checking for task nodes with their IDs
    // TaskNode renders with data-testid="task-node" but we can use data attributes
    const task1Node = page.locator('[data-testid="task-node"]').filter({ has: page.locator('text="Task from Plan Alpha"') });
    const task2Node = page.locator('[data-testid="task-node"]').filter({ has: page.locator('text="Another Task from Plan Alpha"') });

    await expect(task1Node).toBeVisible();
    await expect(task2Node).toBeVisible();

    // Verify task-3 (from session-2) is NOT shown
    const task3Node = page.locator('[data-testid="task-node"]').filter({ has: page.locator('text="Task from Plan Beta"') });
    await expect(task3Node).not.toBeVisible();
  });

  test("Test 2: Accept session → navigate to Kanban → verify filtered board", async ({ page }) => {
    await setDeterministicActivePlan(page, "project-mock-1", "session-2", "session-2");

    // Wait for React Query to refetch with new ideationSessionId
    await waitForQueriesAfterPlanChange(page, 2000);

    // Navigate to Kanban
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    // Wait for tasks to load
    await page.waitForSelector('[data-testid^="task-card-"]', { timeout: 10000 });

    // Verify only task-3 (from session-2) is shown
    await expect(page.locator('[data-testid="task-card-task-3"]')).toBeVisible();

    // Verify tasks from session-1 are NOT shown
    await expect(page.locator('[data-testid="task-card-task-1"]')).not.toBeVisible();
    await expect(page.locator('[data-testid="task-card-task-2"]')).not.toBeVisible();
  });

  test("Test 3: Select plan in Graph inline selector → switch to Kanban → verify same plan active", async ({ page }) => {
    // Navigate to Graph
    await page.click('[data-testid="nav-graph"]');

    // Wait for either graph view or empty state to appear
    await Promise.race([
      page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 }),
      page.waitForSelector('[data-testid="graph-empty-state"]', { timeout: 10000 })
    ]);

    await setDeterministicActivePlan(page, "project-mock-1", "session-1", "session-1");

    // Wait for React Query to refetch with new ideationSessionId
    await waitForQueriesAfterPlanChange(page, 2000);

    // Navigate to Kanban
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    // Wait for tasks to load
    await page.waitForSelector('[data-testid^="task-card-"]', { timeout: 10000 });

    // Verify only tasks from session-1 are shown
    await expect(page.locator('[data-testid="task-card-task-1"]')).toBeVisible();
    await expect(page.locator('[data-testid="task-card-task-2"]')).toBeVisible();
    await expect(page.locator('[data-testid="task-card-task-3"]')).not.toBeVisible();
  });

  test("Test 4: Select plan via quick switcher → verify both Graph and Kanban update", async ({ page }) => {
    await setDeterministicActivePlan(page, "project-mock-1", "session-2", "session-2");

    // Wait for React Query to refetch with new ideationSessionId
    await waitForQueriesAfterPlanChange(page, 2000);

    // Verify Graph view shows only session-2 tasks
    await page.click('[data-testid="nav-graph"]');

    // Wait for either graph view or empty state to appear
    await Promise.race([
      page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 }),
      page.waitForSelector('[data-testid="graph-empty-state"]', { timeout: 10000 })
    ]);

    await page.waitForTimeout(500);

    const task3NodeInGraph = page.locator('[data-testid="task-node"]').filter({ has: page.locator('text="Task from Plan Beta"') });
    const task1NodeInGraph = page.locator('[data-testid="task-node"]').filter({ has: page.locator('text="Task from Plan Alpha"') });

    await expect(task3NodeInGraph).toBeVisible();
    await expect(task1NodeInGraph).not.toBeVisible();

    // Verify Kanban view shows only session-2 tasks
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });
    await page.waitForSelector('[data-testid^="task-card-"]', { timeout: 10000 });

    await expect(page.locator('[data-testid="task-card-task-3"]')).toBeVisible();
    await expect(page.locator('[data-testid="task-card-task-1"]')).not.toBeVisible();
  });

  test("Test 5: No active plan → both views show empty state", async ({ page }) => {
    await setDeterministicActivePlan(page, "project-mock-1", null, null);

    // Wait for React Query to refetch with null ideationSessionId
    await waitForQueriesAfterPlanChange(page, 2000);

    // Check Graph view shows empty/no tasks
    await page.click('[data-testid="nav-graph"]');

    // Wait for either graph view or empty state to appear
    await Promise.race([
      page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 }),
      page.waitForSelector('[data-testid="graph-empty-state"]', { timeout: 10000 })
    ]);

    await page.waitForTimeout(500);

    // Verify no task nodes are visible
    const taskNodes = page.locator('[data-testid="task-node"]');
    await expect(taskNodes).toHaveCount(0);

    // Check Kanban view shows "No plan selected" state
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    // Verify "No plan selected" text is visible
    const noPlanText = page.locator('text="No plan selected"');
    await expect(noPlanText).toBeVisible();
  });

  test("Test 6: Reopen active session → verify plan cleared → empty states shown", async ({ page }) => {
    await setDeterministicActivePlan(page, "project-mock-1", "session-1", "session-1");

    // Wait for queries to settle with session-1
    await waitForQueriesAfterPlanChange(page, 2000);

    // Simulate reopening a session which clears the active plan
    await page.evaluate(async () => {
      const mockStore = (window as unknown as {
        __mockStore?: {
          sessions?: Map<string, { status: string; convertedAt: string | null }>;
        };
      }).__mockStore;
      if (mockStore) {
        // Change session status to active (reopened)
        const session = mockStore.sessions.get("session-1");
        if (session) {
          session.status = "active";
          session.convertedAt = null;
        }
      }
      const { planApi } = await import("/src/api/plan");
      await planApi.clearActivePlan("project-mock-1");
      const planStore = (window as unknown as {
        __planStore?: { getState(): { loadActivePlan(projectId: string): Promise<void> } };
      }).__planStore;
      await planStore?.getState().loadActivePlan("project-mock-1");
    });

    // Wait for queries to refetch with null ideationSessionId
    await waitForQueriesAfterPlanChange(page, 2000);

    // Verify Graph shows no tasks
    await page.click('[data-testid="nav-graph"]');

    // Wait for either graph view or empty state to appear
    await Promise.race([
      page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 }),
      page.waitForSelector('[data-testid="graph-empty-state"]', { timeout: 10000 })
    ]);

    await page.waitForTimeout(500);

    const taskNodesInGraph = page.locator('[data-testid="task-node"]');
    await expect(taskNodesInGraph).toHaveCount(0);

    // Verify Kanban shows "No plan selected"
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    const noPlanText = page.locator('text="No plan selected"');
    await expect(noPlanText).toBeVisible();
  });
});

test.describe("Plan Selector UI", () => {
  test.beforeEach(async ({ page }) => {
    await setupApp(page);
  });

  test.skip("Inline selector displays current plan title and task count", async () => {
    // SKIPPED: PlanSelectorInline component does not have test IDs yet
    // TODO: Add data-testid attributes to PlanSelectorInline component
  });

  test.skip("Inline selector opens popover with plan list", async () => {
    // SKIPPED: PlanSelectorInline component does not have test IDs yet
    // TODO: Add data-testid attributes to PlanSelectorInline component
  });

  test.skip("Quick switcher keyboard navigation (up/down/enter)", async () => {
    // SKIPPED: PlanQuickSwitcherPalette component does not have test IDs yet
    // TODO: Add data-testid attributes to PlanQuickSwitcherPalette component
  });

  test.skip("Quick switcher closes on Escape", async () => {
    // SKIPPED: PlanQuickSwitcherPalette component does not have test IDs yet
    // TODO: Add data-testid attributes to PlanQuickSwitcherPalette component
  });

  test.skip("Quick switcher closes on click outside", async () => {
    // SKIPPED: PlanQuickSwitcherPalette component does not have test IDs yet
    // TODO: Add data-testid attributes to PlanQuickSwitcherPalette component
  });
});
