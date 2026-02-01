import { Page } from "@playwright/test";

/**
 * Trigger TaskFullView in web mode by directly manipulating uiStore
 *
 * In web mode, TaskFullView is controlled by uiStore.taskFullViewId.
 * This helper directly opens the full view to bypass the need for a natural UI trigger.
 *
 * RATIONALE: TaskFullView is opened programmatically via openTaskFullView(),
 * but testing requires direct store manipulation in web mode.
 */
export async function openTaskFullView(
  page: Page,
  taskId: string
): Promise<void> {
  // Open TaskFullView via uiStore
  await page.evaluate((id) => {
    const uiStore = (window as any).__uiStore;
    if (uiStore && typeof uiStore.getState === "function") {
      uiStore.getState().openTaskFullView(id);
    } else {
      throw new Error("uiStore not available. Make sure app is running in web mode.");
    }
  }, taskId);

  // Wait for React to process the state change
  await page.waitForTimeout(300);
}

/**
 * Close TaskFullView
 */
export async function closeTaskFullView(page: Page): Promise<void> {
  await page.evaluate(() => {
    const uiStore = (window as any).__uiStore;
    if (uiStore && typeof uiStore.getState === "function") {
      uiStore.getState().closeTaskFullView();
    }
  });

  await page.waitForTimeout(200);
}
