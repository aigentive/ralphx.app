import { test, expect } from "@playwright/test";
import { IdeationPage } from "../../../pages/ideation.page";
import { ProposalEditModalPage } from "../../../pages/modals/proposal-edit.page";
import { setupIdeation } from "../../../fixtures/setup.fixtures";
import { loadMockIdeationSession } from "../../../helpers/ideation.helpers";

/**
 * Visual regression tests for ProposalEditModal component.
 *
 * The ProposalEditModal allows users to edit task proposal details including
 * title, description, category, priority, complexity, steps, and acceptance criteria.
 *
 * These tests run against web mode dev server (npm run dev:web) which uses
 * mock data from src/api-mock/ instead of the real Tauri backend.
 */

test.describe("ProposalEditModal", () => {
  let ideation: IdeationPage;
  let proposalEditModal: ProposalEditModalPage;

  test.beforeEach(async ({ page }) => {
    ideation = new IdeationPage(page);
    proposalEditModal = new ProposalEditModalPage(page);
    await setupIdeation(page);
    await loadMockIdeationSession(page);
  });

  test("opens modal when edit button is clicked on proposal card", async ({ page }) => {
    // Initially modal should not be visible
    await expect(proposalEditModal.modal).not.toBeVisible();

    // Find the first proposal card and hover to reveal edit button
    const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();
    await firstProposalCard.hover();

    // Click the edit button (FileEdit icon)
    await firstProposalCard.getByRole("button").filter({ hasText: /edit/i }).first().click();

    // Modal should now be visible
    await proposalEditModal.waitForModal();
    await expect(proposalEditModal.modal).toBeVisible();
    await expect(proposalEditModal.title).toHaveText("Edit Proposal");
  });

  test("displays all form fields with existing proposal data", async ({ page }) => {
    // Open the modal
    const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();
    await firstProposalCard.hover();
    await firstProposalCard.getByRole("button").filter({ hasText: /edit/i }).first().click();
    await proposalEditModal.waitForModal();

    // All form fields should be visible
    await expect(proposalEditModal.titleInput).toBeVisible();
    await expect(proposalEditModal.descriptionInput).toBeVisible();
    await expect(proposalEditModal.categorySelect).toBeVisible();
    await expect(proposalEditModal.prioritySelect).toBeVisible();

    // Title should be pre-filled with existing data
    await expect(proposalEditModal.titleInput).not.toBeEmpty();
  });

  test("allows editing proposal fields", async ({ page }) => {
    // Open the modal
    const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();
    await firstProposalCard.hover();
    await firstProposalCard.getByRole("button").filter({ hasText: /edit/i }).first().click();
    await proposalEditModal.waitForModal();

    // Edit title
    await proposalEditModal.fillTitle("Updated Proposal Title");
    await expect(proposalEditModal.titleInput).toHaveValue("Updated Proposal Title");

    // Edit description
    await proposalEditModal.fillDescription("Updated description text");
    await expect(proposalEditModal.descriptionInput).toHaveValue("Updated description text");

    // Change category
    await proposalEditModal.selectCategory("testing");
    await expect(proposalEditModal.categorySelect).toHaveValue("testing");
  });

  test("allows adding and editing implementation steps", async ({ page }) => {
    // Open the modal
    const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();
    await firstProposalCard.hover();
    await firstProposalCard.getByRole("button").filter({ hasText: /edit/i }).first().click();
    await proposalEditModal.waitForModal();

    // Get initial step count
    const initialStepCount = await proposalEditModal.getStepCount();

    // Add a new step
    await proposalEditModal.addStep("New implementation step");

    // Step count should increase
    const newStepCount = await proposalEditModal.getStepCount();
    expect(newStepCount).toBe(initialStepCount + 1);
  });

  test("allows adding and editing acceptance criteria", async ({ page }) => {
    // Open the modal
    const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();
    await firstProposalCard.hover();
    await firstProposalCard.getByRole("button").filter({ hasText: /edit/i }).first().click();
    await proposalEditModal.waitForModal();

    // Get initial criterion count
    const initialCriterionCount = await proposalEditModal.getCriterionCount();

    // Add a new criterion
    await proposalEditModal.addCriterion("New acceptance criterion");

    // Criterion count should increase
    const newCriterionCount = await proposalEditModal.getCriterionCount();
    expect(newCriterionCount).toBe(initialCriterionCount + 1);
  });

  test("closes modal when cancel button is clicked", async ({ page }) => {
    // Open the modal
    const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();
    await firstProposalCard.hover();
    await firstProposalCard.getByRole("button").filter({ hasText: /edit/i }).first().click();
    await proposalEditModal.waitForModal();

    // Click cancel
    await proposalEditModal.cancel();

    // Modal should be hidden
    await expect(proposalEditModal.modal).not.toBeVisible();
  });

  test("matches snapshot", async ({ page }) => {
    // Open the modal
    const firstProposalCard = page.locator('[data-testid^="proposal-card-"]').first();
    await firstProposalCard.hover();
    await firstProposalCard.getByRole("button").filter({ hasText: /edit/i }).first().click();
    await proposalEditModal.waitForModal();

    // Wait for animations to complete
    await proposalEditModal.waitForAnimations();

    // Take snapshot
    await expect(page).toHaveScreenshot("proposal-edit-modal.png", {
      maxDiffPixelRatio: 0.01,
    });
  });
});
