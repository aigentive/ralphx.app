import { Page } from "@playwright/test";
import type { AskUserQuestionPayload } from "@/types/ask-user-question";

/**
 * Trigger an AskUserQuestion modal in web mode by emitting an event
 *
 * In web mode, the app uses MockEventBus which is accessible via window.__eventBus.
 * This helper emits the event to trigger the modal programmatically.
 */
export async function triggerAskUserQuestionModal(
  page: Page,
  question: AskUserQuestionPayload
): Promise<void> {
  await page.evaluate((payload) => {
    // Access the global event bus (set by EventProvider in web mode)
    const eventBus = (window as any).__eventBus;
    if (eventBus && typeof eventBus.emit === "function") {
      eventBus.emit("agent:ask_user_question", payload);
    } else {
      throw new Error("EventBus not available. Make sure app is running in web mode.");
    }
  }, question);

  // Wait a small amount of time for React to process the event and update state
  await page.waitForTimeout(100);
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
