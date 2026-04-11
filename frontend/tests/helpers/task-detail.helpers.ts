import { Page } from "@playwright/test";
import type { Task } from "@/types/task";

/**
 * Trigger the task detail overlay in web mode by seeding a task into the
 * mock store and selecting it through the real uiStore path.
 *
 * The current product surface is TaskDetailOverlay, which is rendered when
 * `selectedTaskId` is set for the active kanban project.
 */
export async function openTaskDetailModal(
  page: Page,
  task: Task
): Promise<void> {
  await page.evaluate((taskData) => {
    const uiStore = window.__uiStore as
      | {
          getState(): {
            setCurrentView(view: string): void;
            setSelectedTaskId(taskId: string | null): void;
          };
        }
      | undefined;
    const mockStore = window.__mockStore as
      | {
          projects: Map<string, { id: string }>;
          tasks: Map<string, Task>;
        }
      | undefined;

    if (!uiStore || typeof uiStore.getState !== "function") {
      throw new Error("uiStore not available. Make sure app is running in web mode.");
    }
    if (!mockStore) {
      throw new Error("Mock store not available. Make sure app is running in web mode.");
    }

    const activeProjectId = mockStore.projects.values().next().value?.id;
    if (!activeProjectId) {
      throw new Error("No active mock project available");
    }

    const normalizedTask: Task = {
      ...taskData,
      projectId: activeProjectId,
    };

    mockStore.tasks.set(normalizedTask.id, normalizedTask);

    const state = uiStore.getState();
    state.setCurrentView("kanban");
    state.setSelectedTaskId(normalizedTask.id);
  }, task);

  await page.waitForSelector('[data-testid="task-detail-overlay"]', {
    timeout: 5000,
  });
}

/**
 * Close the task detail overlay.
 */
export async function closeTaskDetailModal(page: Page): Promise<void> {
  await page.evaluate(() => {
    const uiStore = window.__uiStore as
      | {
          getState(): {
            setSelectedTaskId(taskId: string | null): void;
          };
        }
      | undefined;
    if (uiStore && typeof uiStore.getState === "function") {
      uiStore.getState().setSelectedTaskId(null);
    }
  });

  await page.waitForSelector('[data-testid="task-detail-overlay"]', {
    state: "hidden",
    timeout: 5000,
  });
}
