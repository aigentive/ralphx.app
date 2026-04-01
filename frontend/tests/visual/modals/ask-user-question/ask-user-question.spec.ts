import { test, expect } from "@playwright/test";
import { AskUserQuestionModalPage } from "../../../pages/modals/ask-user-question.page";
import { setupApp } from "../../../fixtures/setup.fixtures";
import {
  triggerAskUserQuestionModal,
  createSingleSelectQuestion,
  createMultiSelectQuestion,
} from "../../../helpers/ask-user-question.helpers";

/**
 * Visual regression tests for AskUserQuestionModal component.
 *
 * The AskUserQuestionModal is a modal that displays questions from agents
 * requiring user input during execution. It supports both single-select
 * (radio buttons) and multi-select (checkboxes) options, with an always-present
 * "Other" option for custom text responses.
 */

test.describe("AskUserQuestionModal", () => {
  let modalPage: AskUserQuestionModalPage;

  test.beforeEach(async ({ page }) => {
    modalPage = new AskUserQuestionModalPage(page);
    await setupApp(page);
  });

  test.describe("single-select question", () => {
    test("renders modal with question and options", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);

      // Wait for modal to appear
      await modalPage.waitForModal();

      // Verify modal is visible
      await expect(modalPage.modal).toBeVisible();

      // Verify header and question text
      await expect(modalPage.header).toHaveText(question.header);
      await expect(modalPage.questionText).toHaveText(question.question);

      // Verify all options are rendered as radio buttons
      // 3 options + 1 "Other" option = 4 radio buttons
      const radioCount = await modalPage.getRadioCount();
      expect(radioCount).toBe(4);
    });

    test("allows selecting a single option", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select the first option
      await modalPage.selectOption("JWT tokens");

      // Verify the radio is checked
      const jwtRadio = modalPage.getRadioByLabel("JWT tokens");
      await expect(jwtRadio).toHaveAttribute("data-state", "checked");

      // Verify submit button is enabled
      await expect(modalPage.submitButton).toBeEnabled();
    });

    test("shows text input when Other is selected", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select "Other"
      const otherRadio = modalPage.getOtherRadio();
      await otherRadio.click();

      // Verify text input is visible
      await expect(modalPage.otherInput).toBeVisible();

      // Type custom text
      await modalPage.otherInput.fill("Custom authentication method");

      // Verify submit button is enabled
      await expect(modalPage.submitButton).toBeEnabled();
    });

    test("matches snapshot - default state", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Wait for animations to complete
      await modalPage.waitForAnimations();

      // Take snapshot
      await expect(page).toHaveScreenshot("ask-user-question-single-select.png", {
        maxDiffPixelRatio: 0.01,
      });
    });

    test("matches snapshot - with selection", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select an option
      await modalPage.selectOption("JWT tokens");

      // Wait for animations
      await modalPage.waitForAnimations();

      // Take snapshot
      await expect(page).toHaveScreenshot("ask-user-question-single-select-selected.png", {
        maxDiffPixelRatio: 0.01,
      });
    });

    test("matches snapshot - Other selected with text", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select Other and type text
      await modalPage.selectOtherWithText("Custom authentication method");

      // Wait for animations
      await modalPage.waitForAnimations();

      // Take snapshot
      await expect(page).toHaveScreenshot("ask-user-question-single-select-other.png", {
        maxDiffPixelRatio: 0.01,
      });
    });
  });

  test.describe("multi-select question", () => {
    test("renders modal with checkboxes", async ({ page }) => {
      const question = createMultiSelectQuestion();
      await triggerAskUserQuestionModal(page, question);

      // Wait for modal to appear
      await modalPage.waitForModal();

      // Verify modal is visible
      await expect(modalPage.modal).toBeVisible();

      // Verify header and question text
      await expect(modalPage.header).toHaveText(question.header);
      await expect(modalPage.questionText).toHaveText(question.question);

      // Verify all options are rendered as checkboxes
      // 3 options + 1 "Other" option = 4 checkboxes
      const checkboxCount = await modalPage.getCheckboxCount();
      expect(checkboxCount).toBe(4);
    });

    test("allows selecting multiple options", async ({ page }) => {
      const question = createMultiSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select multiple options
      await modalPage.selectMultiple(["Dark mode", "Analytics"]);

      // Verify both checkboxes are checked
      const darkModeCheckbox = modalPage.getCheckboxByLabel("Dark mode");
      const analyticsCheckbox = modalPage.getCheckboxByLabel("Analytics");
      await expect(darkModeCheckbox).toHaveAttribute("data-state", "checked");
      await expect(analyticsCheckbox).toHaveAttribute("data-state", "checked");

      // Verify submit button is enabled
      await expect(modalPage.submitButton).toBeEnabled();
    });

    test("allows toggling checkboxes", async ({ page }) => {
      const question = createMultiSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Check a checkbox
      await modalPage.toggleCheckbox("Dark mode");
      const checkbox = modalPage.getCheckboxByLabel("Dark mode");
      await expect(checkbox).toHaveAttribute("data-state", "checked");

      // Uncheck it
      await modalPage.toggleCheckbox("Dark mode");
      await expect(checkbox).toHaveAttribute("data-state", "unchecked");
    });

    test("matches snapshot - default state", async ({ page }) => {
      const question = createMultiSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Wait for animations
      await modalPage.waitForAnimations();

      // Take snapshot
      await expect(page).toHaveScreenshot("ask-user-question-multi-select.png", {
        maxDiffPixelRatio: 0.01,
      });
    });

    test("matches snapshot - multiple selections", async ({ page }) => {
      const question = createMultiSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select multiple options
      await modalPage.selectMultiple(["Dark mode", "Notifications"]);

      // Wait for animations
      await modalPage.waitForAnimations();

      // Take snapshot
      await expect(page).toHaveScreenshot("ask-user-question-multi-select-selected.png", {
        maxDiffPixelRatio: 0.01,
      });
    });
  });

  test.describe("submit button behavior", () => {
    test("submit button is disabled when no option is selected", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Verify submit button is disabled
      await expect(modalPage.submitButton).toBeDisabled();
    });

    test("submit button is disabled when Other is selected but input is empty", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select Other but don't type anything
      const otherRadio = modalPage.getOtherRadio();
      await otherRadio.click();

      // Verify submit button is still disabled
      await expect(modalPage.submitButton).toBeDisabled();
    });

    test("submit button is enabled when Other has valid text", async ({ page }) => {
      const question = createSingleSelectQuestion();
      await triggerAskUserQuestionModal(page, question);
      await modalPage.waitForModal();

      // Select Other and type text
      await modalPage.selectOtherWithText("Custom value");

      // Verify submit button is enabled
      await expect(modalPage.submitButton).toBeEnabled();
    });
  });
});
