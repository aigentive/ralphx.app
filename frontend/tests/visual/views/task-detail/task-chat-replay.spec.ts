import { expect, test } from "@playwright/test";
import { setupTaskChatScenario } from "../../../fixtures/chat.fixtures";

test.describe("Task Chat Replay", () => {
  test("renders DB-derived execution replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "execution_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(panel.locator('[data-testid="chat-session-routing"]')).toContainText(
      "Continuing stored Claude session",
    );
    await expect(
      panel.getByText("Execution replay sampled from a compact two-message worker conversation."),
    ).toBeVisible();
    await expect(
      panel.getByText("frontend/src/components/Chat/MessageItem.tsx"),
    ).toBeVisible();
  });

  test("renders DB-derived review replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "review_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(panel.locator('[data-testid="chat-session-routing"]')).toContainText(
      "Continuing stored Claude session",
    );
    await expect(
      panel.getByText("Reviewer replay sampled from a compact two-message real conversation."),
    ).toBeVisible();
    await expect(panel.getByText(/Changes Requested/i)).toBeVisible();
  });

  test("renders DB-derived merge replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "merge_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(panel.locator('[data-testid="chat-session-routing"]')).toContainText(
      "Continuing stored Codex session",
    );
    await expect(
      panel.getByText("Merge replay sampled from a compact two-message merger conversation."),
    ).toBeVisible();
    await expect(panel.getByText(/get_merge_target/i)).toBeVisible();
  });
});
