import { expect, test, type Locator } from "@playwright/test";
import {
  setupIdeationChatScenario,
  setupTaskChatScenario,
} from "../../../fixtures/chat.fixtures";

async function expandWidget(widget: Locator) {
  await widget.locator('[role="button"]').first().click();
}

test.describe("Chat Widget Matrix", () => {
  test("proposal widget states", async ({ page }) => {
    await setupIdeationChatScenario(page, "ideation_widget_matrix");

    const createWidget = page.locator('[data-testid="proposal-widget-created"]');
    const updateWidget = page.locator('[data-testid="proposal-widget-updated"]');
    const deleteWidget = page.locator('[data-testid="proposal-widget-deleted"]');

    await expect(createWidget).toBeVisible();
    await expect(updateWidget).toBeVisible();
    await expect(deleteWidget).toBeVisible();

    await expect(createWidget).toHaveScreenshot("proposal-widget-created.png");
    await expect(updateWidget).toHaveScreenshot("proposal-widget-updated.png");
    await expect(deleteWidget).toHaveScreenshot("proposal-widget-deleted.png");
  });

  test("verification widget states", async ({ page }) => {
    await setupIdeationChatScenario(page, "ideation_widget_matrix");

    const updateWidget = page.locator('[data-testid="verification-widget-update"]');
    const getWidget = page.locator('[data-testid="verification-widget-get"]');
    const pendingWidget = page.locator('[data-testid="verification-widget-pending"]');

    await expect(updateWidget).toBeVisible();
    await expect(getWidget).toBeVisible();
    await expect(pendingWidget).toBeVisible();

    await expect(updateWidget).toHaveScreenshot("verification-widget-update.png");
    await expect(getWidget).toHaveScreenshot("verification-widget-get.png");
    await expect(pendingWidget).toHaveScreenshot("verification-widget-pending.png");
  });

  test("send message and ideation widget states", async ({ page }) => {
    await setupIdeationChatScenario(page, "ideation_widget_matrix");

    const sendMessageWidget = page.locator('[data-testid="send-message-widget-broadcast"]');
    const askQuestionWidget = page.locator('[data-testid="ideation-widget-ask-question"]');
    const createPlanWidget = page.locator('[data-testid="ideation-widget-create-plan"]');
    const updatePlanWidget = page.locator('[data-testid="ideation-widget-update-plan"]');

    await expect(sendMessageWidget).toBeVisible();
    await expect(askQuestionWidget).toBeVisible();
    await expect(createPlanWidget).toBeVisible();
    await expect(updatePlanWidget).toBeVisible();

    await sendMessageWidget.getByRole("button").click();

    await expect(sendMessageWidget).toHaveScreenshot("send-message-widget-broadcast.png");
    await expect(askQuestionWidget).toHaveScreenshot("ideation-widget-ask-question.png");
    await expect(createPlanWidget).toHaveScreenshot("ideation-widget-create-plan.png");
    await expect(updatePlanWidget).toHaveScreenshot("ideation-widget-update-plan.png");
  });

  test("child session widget states", async ({ page }) => {
    await setupIdeationChatScenario(page, "ideation_widget_matrix", {
      childSessionOverrides: {
        "child-session-loading-1": { delayMs: 10_000 },
        "child-session-error-1": {
          error: "Unable to load child session in visual test",
        },
      },
    });

    const activeWidget = page.locator('[data-testid="child-session-widget-active"]').first();
    const pendingWidget = page.locator('[data-testid="child-session-widget-pending"]').first();
    const loadingWidget = page.locator('[data-testid="child-session-widget-loading"]').first();
    const errorWidget = page.locator('[data-testid="child-session-widget-error"]').first();

    await expect(activeWidget).toBeVisible();
    await expect(pendingWidget).toBeVisible();
    await expect(loadingWidget).toBeVisible();
    await expect(errorWidget).toBeVisible();

    await expandWidget(activeWidget);
    await expandWidget(loadingWidget);
    await expandWidget(errorWidget);

    await expect(activeWidget).toHaveScreenshot("child-session-widget-active.png");
    await expect(pendingWidget).toHaveScreenshot("child-session-widget-pending.png");
    await expect(loadingWidget).toHaveScreenshot("child-session-widget-loading.png");
    await expect(errorWidget).toHaveScreenshot("child-session-widget-error.png");
  });

  test("review widget states", async ({ page }) => {
    await setupTaskChatScenario(page, "review_widget_matrix");

    const completeWidget = page.locator('[data-testid="review-widget-complete"]');
    const notesWidget = page.locator('[data-testid="review-widget-notes"]');

    await expect(completeWidget).toBeVisible();
    await expect(notesWidget).toBeVisible();

    await completeWidget.click();
    await notesWidget.getByRole("button").click();

    await expect(completeWidget).toHaveScreenshot("review-widget-complete.png");
    await expect(notesWidget).toHaveScreenshot("review-widget-notes.png");
  });

  test("merge widget states", async ({ page }) => {
    await setupTaskChatScenario(page, "merge_widget_matrix");

    const targetWidget = page.locator('[data-testid="merge-widget-target"]');
    const conflictWidget = page.locator('[data-testid="merge-widget-conflict"]');
    const incompleteWidget = page.locator('[data-testid="merge-widget-incomplete"]');
    const completeWidget = page.locator('[data-testid="merge-widget-complete"]');

    await expect(targetWidget).toBeVisible();
    await expect(conflictWidget).toBeVisible();
    await expect(incompleteWidget).toBeVisible();
    await expect(completeWidget).toBeVisible();

    await conflictWidget.getByRole("button").click();
    await incompleteWidget.getByRole("button").click();

    await expect(targetWidget).toHaveScreenshot("merge-widget-target.png");
    await expect(conflictWidget).toHaveScreenshot("merge-widget-conflict.png");
    await expect(incompleteWidget).toHaveScreenshot("merge-widget-incomplete.png");
    await expect(completeWidget).toHaveScreenshot("merge-widget-complete.png");
  });
});
