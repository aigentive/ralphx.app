import { Page } from "@playwright/test";
import type { Task } from "@/types/task";

/**
 * Trigger TaskDetailModal in web mode by directly manipulating uiStore
 *
 * In web mode, the modal's visibility is controlled by uiStore.activeModal === "task-detail"
 * and uiStore.modalContext.task. This helper directly opens the modal to bypass the need
 * for a natural UI trigger (which doesn't exist yet).
 *
 * RATIONALE: TaskDetailModal was designed to be opened programmatically via openModal(),
 * but no UI trigger (button, context menu) exists to open it. Direct store manipulation
 * allows visual testing without implementing a production UI trigger first.
 */
export async function openTaskDetailModal(
  page: Page,
  task: Task
): Promise<void> {
  // Open task-detail modal via uiStore
  await page.evaluate((taskData) => {
    const uiStore = (window as any).__uiStore;
    if (uiStore && typeof uiStore.getState === "function") {
      uiStore.getState().openModal("task-detail", { task: taskData });
    } else {
      throw new Error("uiStore not available. Make sure app is running in web mode.");
    }
  }, task);

  // Wait for React to process the state change
  await page.waitForTimeout(200);
}

/**
 * Close TaskDetailModal
 */
export async function closeTaskDetailModal(page: Page): Promise<void> {
  await page.evaluate(() => {
    const uiStore = (window as any).__uiStore;
    if (uiStore && typeof uiStore.getState === "function") {
      uiStore.getState().closeModal();
    }
  });

  await page.waitForTimeout(200);
}
