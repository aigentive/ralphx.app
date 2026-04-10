import { expect, test } from "@playwright/test";
import { setupIdeationChatScenario } from "../../../fixtures/chat.fixtures";

test.describe("Ideation Chat Replay", () => {
  test.beforeEach(async ({ page }) => {
    await setupIdeationChatScenario(page, "ideation_db_widget_mix");
  });

  test("renders DB-derived chat replay widgets in the ideation conversation panel", async ({ page }) => {
    const panel = page.locator('[data-testid="conversation-panel"]');

    await expect(panel).toBeVisible();
    await expect(panel.getByText(/Continuing stored Claude session/i)).toBeVisible();
    await expect(panel.getByText("Preferred default for automatic PR creation?")).toBeVisible();
    await expect(panel.getByText("to layer1-critic")).toBeVisible();
    await expect(panel.getByText("src-tauri/src/application/chat_service/mod.rs")).toBeVisible();
  });

  test("matches ideation chat replay snapshot", async ({ page }) => {
    const panel = page.locator('[data-testid="conversation-panel"]');
    await expect(panel).toBeVisible();
    await expect(panel.getByText(/Continuing stored Claude session/i)).toBeVisible();
    await expect(panel.getByText("Preferred default for automatic PR creation?")).toBeVisible();
    await expect(panel.getByText("to layer1-critic")).toBeVisible();
    await expect(panel.getByText("src-tauri/src/application/chat_service/mod.rs")).toBeVisible();
    await expect(panel).toHaveScreenshot("ideation-chat-replay.png", {
      maxDiffPixelRatio: 0.01,
    });
  });
});
