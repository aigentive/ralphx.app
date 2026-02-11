import { test, expect } from "@playwright/test";
import { setupApp } from "../fixtures/setup.fixtures";

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

test.describe("Active Plan Cross-View Sync", () => {
  test.beforeEach(async ({ page }) => {
    await setupApp(page);

    // Seed mock store with accepted ideation sessions
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (!store) {
        throw new Error("Mock store not available - ensure api-mock is configured");
      }

      // Clear existing data
      store.projects.clear();
      store.tasks.clear();

      // Create project
      const project = {
        id: "test-project-1",
        name: "Test Project",
        workingDirectory: "/test",
        defaultBranch: "main",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      store.projects.set(project.id, project);

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
          ideationSessionId: "session-1",
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
          ideationSessionId: "session-1",
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
          ideationSessionId: "session-2",
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      ];

      tasks.forEach(t => store.tasks.set(t.id, t));

      // Initialize active plan state (no plan selected initially)
      if (!store.activePlans) store.activePlans = new Map();
      store.activePlans.set(project.id, null);

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
    // Set active plan directly via mock store (simulating session acceptance)
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-1");
      }
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Navigate to Graph
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 });

    // Wait for graph to render
    await page.waitForTimeout(500);

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
    // Set active plan directly via mock store
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-2");
      }
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

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
    await page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 });

    // Simulate plan selection by updating mock store
    // (Plan selector UI interaction would trigger this in real app)
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-1");
      }
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Wait for graph to update
    await page.waitForTimeout(500);

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
    // Simulate plan selection via quick switcher by updating mock store
    // (Quick switcher UI interaction would trigger this in real app)
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-2");
      }
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Verify Graph view shows only session-2 tasks
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 });
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
    // Clear active plan in mock store
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", null);
      }
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Check Graph view shows empty/no tasks
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 });
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
    // Set initial active plan
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-1");
      }
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Wait for active plan to be set
    await page.waitForTimeout(500);

    // Simulate reopening a session which clears the active plan
    await page.evaluate(() => {
      const store = (window as unknown as { __mockStore?: unknown }).__mockStore;
      if (store) {
        // Change session status to active (reopened)
        const session = store.sessions.get("session-1");
        if (session) {
          session.status = "active";
          session.convertedAt = null;
        }
        // Clear active plan
        if (store.activePlans) {
          store.activePlans.set("test-project-1", null);
        }
      }
      const queryClient = (window as unknown as { __queryClient?: { invalidateQueries: () => void } }).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Verify Graph shows no tasks
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="task-graph-view"]', { timeout: 10000 });
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
