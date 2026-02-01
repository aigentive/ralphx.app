/**
 * Test Helpers: TaskPickerDialog
 *
 * Utilities for triggering and interacting with TaskPickerDialog in visual tests.
 */

import { Page } from "@playwright/test";

/**
 * Opens TaskPickerDialog by navigating to ideation view and clicking trigger
 */
export async function openTaskPickerDialog(page: Page): Promise<void> {
  // Navigate to base URL first
  await page.goto("http://localhost:5173/");

  // Wait for app to load
  await page.waitForLoadState("networkidle");

  // Click the Ideation navigation button
  const ideationNav = page.getByRole("button", { name: "Ideation" });
  await ideationNav.click();

  // Wait for ideation view to appear - look for the "Seed from Draft Task" button
  await page.waitForSelector('button:has-text("Seed from Draft Task")', { timeout: 5000 });

  // Click "Seed from Draft Task" button to open TaskPickerDialog
  const seedButton = page.getByRole("button", { name: "Seed from Draft Task" });
  await seedButton.click();

  // Wait for dialog to appear
  await page.waitForSelector('[role="dialog"]', { timeout: 3000 });
}

/**
 * Opens TaskPickerDialog directly via window manipulation (for isolated testing)
 */
export async function openTaskPickerDialogDirect(page: Page): Promise<void> {
  await page.goto("http://localhost:5173/ideation");

  // Wait for React to mount
  await page.waitForTimeout(500);

  // Trigger dialog via state manipulation (if exposed to window)
  await page.evaluate(() => {
    // This requires the component to be controlled by a state exposed to window
    // If not available, we'll need to use the natural trigger method above
    const event = new CustomEvent("open-task-picker");
    window.dispatchEvent(event);
  });
}
