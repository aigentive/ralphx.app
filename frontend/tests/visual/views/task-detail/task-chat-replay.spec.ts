import { expect, test } from "@playwright/test";
import { setupTaskChatScenario } from "../../../fixtures/chat.fixtures";

test.describe("Task Chat Replay", () => {
  test("renders DB-derived execution replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "execution_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(page.getByTestId("chat-session-provider-badge")).toHaveText(/Claude/i);
    await expect(
      panel.getByText("Execution replay sampled from a compact two-message worker conversation."),
    ).toBeVisible();
    await expect(
      panel.getByText("frontend/src/components/Chat/MessageItem.tsx"),
    ).toBeVisible();

    await page.getByTestId("chat-session-stats-button").click();
    await expect(page.getByText("980")).toBeVisible();
    await expect(page.getByText("164")).toBeVisible();
  });

  test("renders DB-derived review replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "review_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(page.getByTestId("chat-session-provider-badge")).toHaveText(/Claude/i);
    await expect(
      panel.getByText("Reviewer replay sampled from a compact two-message real conversation."),
    ).toBeVisible();
    await expect(panel.getByText(/Changes Requested/i)).toBeVisible();

    await page.getByTestId("chat-session-stats-button").click();
    await expect(page.getByText("1,506")).toBeVisible();
    await expect(page.getByText("203")).toBeVisible();
  });

  test("renders DB-derived merge replay in the task chat panel", async ({ page }) => {
    await setupTaskChatScenario(page, "merge_db_compact");

    const panel = page.locator('[data-testid="integrated-chat-panel"]');

    await expect(panel).toBeVisible();
    await expect(page.getByTestId("chat-session-provider-badge")).toHaveText(/Codex/i);
    await expect(
      panel.getByText("Merge replay sampled from a compact two-message merger conversation."),
    ).toBeVisible();

    await page.getByTestId("chat-session-stats-button").click();
    await expect(page.getByText("1,244")).toBeVisible();
    await expect(page.getByText("188")).toBeVisible();
    await expect(page.getByText("gpt-5.4", { exact: true })).toBeVisible();
  });
});
