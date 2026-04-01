import { Page } from "@playwright/test";
import type { AskUserQuestionPayload } from "@/types/ask-user-question";

/**
 * Trigger an AskUserQuestion modal in web mode by directly manipulating uiStore
 *
 * In web mode, the modal's visibility is controlled by uiStore.activeQuestion.
 * This helper directly sets the activeQuestion state to bypass the event subscription
 * timing issue.
 *
 * RATIONALE: The event-based approach has a race condition where the event is emitted
 * before useAskUserQuestion's useEffect subscribes. Direct store manipulation is more
 * reliable for testing.
 */
export async function triggerAskUserQuestionModal(
  page: Page,
  question: AskUserQuestionPayload
): Promise<void> {
  // Directly set activeQuestion in uiStore
  await page.evaluate((payload) => {
    // Access zustand store directly from window (exposed by devtools)
    // In production, this would be triggered by event → hook → store
    const uiStore = (window as any).__uiStore;
    if (uiStore && typeof uiStore.getState === "function") {
      uiStore.getState().setActiveQuestion(payload);
    } else {
      throw new Error("uiStore not available. Make sure app is running in web mode.");
    }
  }, question);

  // Wait for React to process the state change
  await page.waitForTimeout(200);
}

/**
 * Create a sample single-select question for testing
 */
export function createSingleSelectQuestion(): AskUserQuestionPayload {
  return {
    taskId: "test-task-123",
    header: "Authentication Method",
    question: "Which authentication method should we use?",
    options: [
      { label: "JWT tokens", description: "Recommended for APIs" },
      { label: "Session cookies", description: "Traditional web sessions" },
      { label: "OAuth only", description: "Third-party auth providers" },
    ],
    multiSelect: false,
  };
}

/**
 * Create a sample multi-select question for testing
 */
export function createMultiSelectQuestion(): AskUserQuestionPayload {
  return {
    taskId: "test-task-456",
    header: "Features to Enable",
    question: "Which features do you want to enable?",
    options: [
      { label: "Dark mode", description: "Enable dark theme support" },
      { label: "Analytics", description: "Track user behavior" },
      { label: "Notifications", description: "Push notification support" },
    ],
    multiSelect: true,
  };
}
