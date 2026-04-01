import { test, expect } from "@playwright/test";
import { setupApp } from "../../../fixtures/setup.fixtures";

/**
 * Minimal test to verify AskUserQuestionModal can be triggered
 */

test("can trigger ask user question modal via event bus", async ({ page }) => {
  // Listen to console errors
  const consoleErrors: string[] = [];
  page.on('console', msg => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  await setupApp(page);

  // Force a hard reload to ensure latest code
  await page.reload({ waitUntil: "networkidle" });
  await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });

  // Wait for React to finish mounting and hooks to subscribe
  await page.waitForTimeout(2000);

  // Check for console errors
  if (consoleErrors.length > 0) {
    console.log("Console errors detected:", consoleErrors);
  }

  // Check if event bus is available
  const hasEventBus = await page.evaluate(() => {
    return typeof (window as any).__eventBus !== "undefined";
  });

  console.log("EventBus available:", hasEventBus);

  if (!hasEventBus) {
    throw new Error("EventBus not available on window");
  }

  // Check listener count
  const listenerCount = await page.evaluate(() => {
    const bus = (window as any).__eventBus;
    return bus.getListenerCount ? bus.getListenerCount("agent:ask_user_question") : -1;
  });

  console.log("Listeners for agent:ask_user_question:", listenerCount);

  // Emit the event
  await page.evaluate(() => {
    const payload = {
      taskId: "test-123",
      header: "Test Header",
      question: "Test question?",
      options: [
        { label: "Option 1", description: "First option" },
        { label: "Option 2", description: "Second option" },
      ],
      multiSelect: false,
    };

    (window as any).__eventBus.emit("agent:ask_user_question", payload);
  });

  // Wait a bit for React to process
  await page.waitForTimeout(500);

  // Check if modal appears
  const modal = page.locator('[data-testid="ask-user-question-modal"]');
  await expect(modal).toBeVisible({ timeout: 5000 });
});
