import { Page } from "@playwright/test";

/**
 * Helper to trigger BlockReasonDialog in web mode
 *
 * Since BlockReasonDialog is opened via local state in TaskCardContextMenu,
 * we need to either:
 * 1. Navigate the full UI flow (kanban → right-click task → click Block)
 * 2. Create a test wrapper component
 *
 * For this test, we'll use approach #1 - the actual production flow.
 * This ensures we're testing the dialog as it's actually used.
 */

/**
 * Navigate to kanban view and open block dialog for a task
 * This uses the actual production UI flow
 */
export async function openBlockDialogViaKanban(
  page: Page,
  projectId?: string
): Promise<void> {
  // Navigate to kanban if not already there
  const kanbanButton = page.getByRole("button", { name: /kanban/i });
  if (await kanbanButton.isVisible()) {
    await kanbanButton.click();
    await page.waitForTimeout(500);
  }

  // Find the specific "Ready Task" card (we know this exists in seed data)
  // The task card contains the title text
  const readyTaskCard = page.getByText("Ready Task").locator("..");

  // Right-click to open context menu
  await readyTaskCard.click({ button: "right" });
  await page.waitForTimeout(300);

  // Click "Block" menu item
  const blockMenuItem = page.getByRole("menuitem", { name: /^Block$/i });
  await blockMenuItem.click();
  await page.waitForTimeout(200);
}
