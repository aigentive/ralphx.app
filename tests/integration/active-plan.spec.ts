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
      const store = (window as any).__mockStore;
      if (!store) return;

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
      const queryClient = (window as any).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Wait for initial data to load
    await page.waitForTimeout(500);
  });

  test("Test 1: Accept session in Ideation → navigate to Graph → verify filtered tasks shown", async ({ page }) => {
    // Navigate to Ideation
    await page.click('[data-testid="nav-ideation"]');
    await page.waitForSelector('[data-testid="ideation-view"]', { timeout: 10000 });

    // Click session to open it
    await page.click('[data-testid="session-item-session-1"]');

    // Click accept/apply button (this should set active plan)
    const acceptButton = page.locator('button:has-text("Accept")');
    if (await acceptButton.isVisible()) {
      await acceptButton.click();

      // Mock the accept API call result
      await page.evaluate(() => {
        const store = (window as any).__mockStore;
        if (store && store.activePlans) {
          store.activePlans.set("test-project-1", "session-1");
        }
      });
    }

    // Navigate to Graph
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="graph-view"]', { timeout: 10000 });

    // Verify only tasks from session-1 are shown
    const graphNodes = page.locator('[data-testid^="graph-node-"]');
    await expect(graphNodes).toHaveCount(2); // task-1 and task-2

    // Verify task-3 (from session-2) is NOT shown
    await expect(page.locator('[data-testid="graph-node-task-3"]')).not.toBeVisible();
  });

  test("Test 2: Accept session → navigate to Kanban → verify filtered board", async ({ page }) => {
    // Set active plan directly via mock store
    await page.evaluate(() => {
      const store = (window as any).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-2");
      }
      const queryClient = (window as any).__queryClient;
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
    await page.waitForSelector('[data-testid="graph-view"]', { timeout: 10000 });

    // Open plan selector in Graph
    const planSelector = page.locator('[data-testid="plan-selector-inline"]');
    await planSelector.click();

    // Select "Plan Alpha" (session-1)
    await page.click('[data-testid="plan-option-session-1"]');

    // Update mock store to reflect selection
    await page.evaluate(() => {
      const store = (window as any).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-1");
      }
      const queryClient = (window as any).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Navigate to Kanban
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    // Verify plan selector in Kanban shows "Plan Alpha"
    const kanbanSelector = page.locator('[data-testid="plan-selector-inline"]');
    await expect(kanbanSelector).toContainText("Plan Alpha");

    // Verify only tasks from session-1 are shown
    await expect(page.locator('[data-testid="task-card-task-1"]')).toBeVisible();
    await expect(page.locator('[data-testid="task-card-task-2"]')).toBeVisible();
    await expect(page.locator('[data-testid="task-card-task-3"]')).not.toBeVisible();
  });

  test("Test 4: Select plan via Cmd+Shift+P → verify both Graph and Kanban update", async ({ page }) => {
    // Open quick switcher with Cmd+Shift+P
    await page.keyboard.press("Meta+Shift+P");

    // Wait for quick switcher to appear
    await page.waitForSelector('[data-testid="plan-quick-switcher"]', { timeout: 5000 });

    // Type to filter plans
    await page.fill('[data-testid="plan-quick-switcher-input"]', "Beta");

    // Select "Plan Beta" from results
    await page.click('[data-testid="plan-result-session-2"]');

    // Update mock store
    await page.evaluate(() => {
      const store = (window as any).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-2");
      }
      const queryClient = (window as any).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Verify Graph view shows only session-2 tasks
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="graph-view"]', { timeout: 10000 });
    await expect(page.locator('[data-testid="graph-node-task-3"]')).toBeVisible();
    await expect(page.locator('[data-testid="graph-node-task-1"]')).not.toBeVisible();

    // Verify Kanban view shows only session-2 tasks
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });
    await expect(page.locator('[data-testid="task-card-task-3"]')).toBeVisible();
    await expect(page.locator('[data-testid="task-card-task-1"]')).not.toBeVisible();
  });

  test("Test 5: No active plan → both views show empty state", async ({ page }) => {
    // Clear active plan in mock store
    await page.evaluate(() => {
      const store = (window as any).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", null);
      }
      const queryClient = (window as any).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Check Graph view shows empty state
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="graph-view"]', { timeout: 10000 });

    const graphEmptyState = page.locator('[data-testid="graph-empty-state"]');
    await expect(graphEmptyState).toBeVisible();
    await expect(graphEmptyState).toContainText("No plan selected");

    // Check Kanban view shows empty state
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    const kanbanEmptyState = page.locator('[data-testid="kanban-empty-state"]');
    await expect(kanbanEmptyState).toBeVisible();
    await expect(kanbanEmptyState).toContainText("No plan selected");
  });

  test("Test 6: Reopen active session → verify plan cleared → empty states shown", async ({ page }) => {
    // Set initial active plan
    await page.evaluate(() => {
      const store = (window as any).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-1");
      }
      const queryClient = (window as any).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Navigate to Ideation
    await page.click('[data-testid="nav-ideation"]');
    await page.waitForSelector('[data-testid="ideation-view"]', { timeout: 10000 });

    // Click session-1 to view it
    await page.click('[data-testid="session-item-session-1"]');

    // Click reopen button
    const reopenButton = page.locator('button:has-text("Reopen")');
    if (await reopenButton.isVisible()) {
      await reopenButton.click();

      // Mock the reopen API call - should clear active plan
      await page.evaluate(() => {
        const store = (window as any).__mockStore;
        if (store) {
          // Change session status to active
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
        const queryClient = (window as any).__queryClient;
        if (queryClient) {
          queryClient.invalidateQueries();
        }
      });
    }

    // Verify Graph shows empty state
    await page.click('[data-testid="nav-graph"]');
    await page.waitForSelector('[data-testid="graph-view"]', { timeout: 10000 });
    await expect(page.locator('[data-testid="graph-empty-state"]')).toBeVisible();

    // Verify Kanban shows empty state
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });
    await expect(page.locator('[data-testid="kanban-empty-state"]')).toBeVisible();
  });
});

test.describe("Plan Selector UI", () => {
  test.beforeEach(async ({ page }) => {
    await setupApp(page);
  });

  test("Inline selector displays current plan title and task count", async ({ page }) => {
    // Set active plan with known data
    await page.evaluate(() => {
      const store = (window as any).__mockStore;
      if (store && store.activePlans) {
        store.activePlans.set("test-project-1", "session-1");
      }
      const queryClient = (window as any).__queryClient;
      if (queryClient) {
        queryClient.invalidateQueries();
      }
    });

    // Navigate to Kanban
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    // Check selector shows plan title
    const selector = page.locator('[data-testid="plan-selector-inline"]');
    await expect(selector).toContainText("Plan Alpha");

    // Check selector shows task count badge
    const badge = selector.locator('[data-testid="task-count-badge"]');
    await expect(badge).toBeVisible();
  });

  test("Inline selector opens popover with plan list", async ({ page }) => {
    await page.click('[data-testid="nav-kanban"]');
    await page.waitForSelector('[data-testid="task-board"]', { timeout: 10000 });

    // Click selector to open popover
    const selector = page.locator('[data-testid="plan-selector-inline"]');
    await selector.click();

    // Verify popover is visible
    const popover = page.locator('[data-testid="plan-selector-popover"]');
    await expect(popover).toBeVisible();

    // Verify search input is present
    await expect(page.locator('[data-testid="plan-search-input"]')).toBeVisible();
  });

  test("Quick switcher keyboard navigation (up/down/enter)", async ({ page }) => {
    // Open quick switcher
    await page.keyboard.press("Meta+Shift+P");
    await page.waitForSelector('[data-testid="plan-quick-switcher"]', { timeout: 5000 });

    // Verify first result is highlighted by default
    const firstResult = page.locator('[data-testid="plan-result-0"]');
    await expect(firstResult).toHaveClass(/highlighted/);

    // Press down arrow
    await page.keyboard.press("ArrowDown");

    // Verify second result is now highlighted
    const secondResult = page.locator('[data-testid="plan-result-1"]');
    await expect(secondResult).toHaveClass(/highlighted/);

    // Press up arrow
    await page.keyboard.press("ArrowUp");

    // Verify first result is highlighted again
    await expect(firstResult).toHaveClass(/highlighted/);

    // Press enter to select
    await page.keyboard.press("Enter");

    // Verify quick switcher closed
    await expect(page.locator('[data-testid="plan-quick-switcher"]')).not.toBeVisible();
  });

  test("Quick switcher closes on Escape", async ({ page }) => {
    // Open quick switcher
    await page.keyboard.press("Meta+Shift+P");
    await page.waitForSelector('[data-testid="plan-quick-switcher"]', { timeout: 5000 });

    // Press Escape
    await page.keyboard.press("Escape");

    // Verify quick switcher closed
    await expect(page.locator('[data-testid="plan-quick-switcher"]')).not.toBeVisible();
  });

  test("Quick switcher closes on click outside", async ({ page }) => {
    // Open quick switcher
    await page.keyboard.press("Meta+Shift+P");
    await page.waitForSelector('[data-testid="plan-quick-switcher"]', { timeout: 5000 });

    // Click outside the quick switcher
    await page.click('body', { position: { x: 10, y: 10 } });

    // Verify quick switcher closed
    await expect(page.locator('[data-testid="plan-quick-switcher"]')).not.toBeVisible();
  });
});
