import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TaskDetailQAPanel } from "./TaskDetailQAPanel";
import type {
  TaskQAResponse,
  QAResultsResponse,
  AcceptanceCriterionResponse,
  QATestStepResponse,
  QAStepResultResponse,
} from "@/lib/tauri";

// Mock data factories
function createAcceptanceCriterion(
  overrides: Partial<AcceptanceCriterionResponse> = {}
): AcceptanceCriterionResponse {
  return {
    id: "AC1",
    description: "User can see the task board with 7 columns",
    testable: true,
    criteria_type: "visual",
    ...overrides,
  };
}

function createQATestStep(overrides: Partial<QATestStepResponse> = {}): QATestStepResponse {
  return {
    id: "QA1",
    criteria_id: "AC1",
    description: "Verify task board renders with correct columns",
    commands: ["agent-browser open http://localhost:1420", "agent-browser snapshot -i -c"],
    expected: "All 7 columns visible",
    ...overrides,
  };
}

function createQAStepResult(overrides: Partial<QAStepResultResponse> = {}): QAStepResultResponse {
  return {
    step_id: "QA1",
    status: "passed",
    screenshot: undefined,
    actual: undefined,
    expected: undefined,
    error: undefined,
    ...overrides,
  };
}

function createTaskQAResponse(overrides: Partial<TaskQAResponse> = {}): TaskQAResponse {
  return {
    id: "qa-1",
    task_id: "task-123",
    acceptance_criteria: [createAcceptanceCriterion()],
    qa_test_steps: [createQATestStep()],
    prep_agent_id: "agent-1",
    prep_started_at: "2026-01-24T10:00:00Z",
    prep_completed_at: "2026-01-24T10:05:00Z",
    actual_implementation: undefined,
    refined_test_steps: undefined,
    refinement_agent_id: undefined,
    refinement_completed_at: undefined,
    test_results: undefined,
    screenshots: [],
    test_agent_id: undefined,
    test_completed_at: undefined,
    created_at: "2026-01-24T10:00:00Z",
    ...overrides,
  };
}

function createQAResultsResponse(overrides: Partial<QAResultsResponse> = {}): QAResultsResponse {
  return {
    task_id: "task-123",
    overall_status: "passed",
    total_steps: 1,
    passed_steps: 1,
    failed_steps: 0,
    steps: [createQAStepResult()],
    ...overrides,
  };
}

describe("TaskDetailQAPanel", () => {
  describe("tab navigation", () => {
    it("renders three tabs: Acceptance Criteria, Test Results, Screenshots", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      expect(screen.getByRole("tab", { name: /acceptance criteria/i })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: /test results/i })).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: /screenshots/i })).toBeInTheDocument();
    });

    it("has Acceptance Criteria tab selected by default", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      const criteriaTab = screen.getByRole("tab", { name: /acceptance criteria/i });
      expect(criteriaTab).toHaveAttribute("aria-selected", "true");
    });

    it("switches to Test Results tab when clicked", async () => {
      const user = userEvent.setup();
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={createQAResultsResponse()} />);

      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByRole("tab", { name: /test results/i })).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("tab", { name: /acceptance criteria/i })).toHaveAttribute("aria-selected", "false");
    });

    it("switches to Screenshots tab when clicked", async () => {
      const user = userEvent.setup();
      render(
        <TaskDetailQAPanel
          taskQA={createTaskQAResponse({ screenshots: ["screenshot1.png"] })}
          results={null}
        />
      );

      await user.click(screen.getByRole("tab", { name: /screenshots/i }));

      expect(screen.getByRole("tab", { name: /screenshots/i })).toHaveAttribute("aria-selected", "true");
    });

    it("shows tab badge counts when data is present", () => {
      const taskQA = createTaskQAResponse({
        acceptance_criteria: [createAcceptanceCriterion(), createAcceptanceCriterion({ id: "AC2" })],
        screenshots: ["s1.png", "s2.png", "s3.png"],
      });
      const results = createQAResultsResponse({ total_steps: 2 });

      render(<TaskDetailQAPanel taskQA={taskQA} results={results} />);

      expect(screen.getByTestId("criteria-count")).toHaveTextContent("2");
      expect(screen.getByTestId("results-count")).toHaveTextContent("2");
      expect(screen.getByTestId("screenshots-count")).toHaveTextContent("3");
    });
  });

  describe("Acceptance Criteria tab", () => {
    it("shows empty state when no criteria", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse({ acceptance_criteria: [] })} results={null} />);

      expect(screen.getByText(/no acceptance criteria/i)).toBeInTheDocument();
    });

    it("renders all acceptance criteria", () => {
      const taskQA = createTaskQAResponse({
        acceptance_criteria: [
          createAcceptanceCriterion({ id: "AC1", description: "First criterion" }),
          createAcceptanceCriterion({ id: "AC2", description: "Second criterion" }),
        ],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);

      expect(screen.getByText("First criterion")).toBeInTheDocument();
      expect(screen.getByText("Second criterion")).toBeInTheDocument();
    });

    it("shows criterion ID badge", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      expect(screen.getByText("AC1")).toBeInTheDocument();
    });

    it("shows criterion type badge", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      expect(screen.getByText("visual")).toBeInTheDocument();
    });

    it("shows testable indicator for testable criteria", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      expect(screen.getByTestId("criterion-testable-AC1")).toBeInTheDocument();
    });

    it("hides testable indicator for non-testable criteria", () => {
      const taskQA = createTaskQAResponse({
        acceptance_criteria: [createAcceptanceCriterion({ id: "AC1", testable: false })],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);

      expect(screen.queryByTestId("criterion-testable-AC1")).not.toBeInTheDocument();
    });

    it("shows checkmark for passed criteria when results available", () => {
      const taskQA = createTaskQAResponse({
        acceptance_criteria: [createAcceptanceCriterion({ id: "AC1" })],
        qa_test_steps: [createQATestStep({ id: "QA1", criteria_id: "AC1" })],
      });
      const results = createQAResultsResponse({
        steps: [createQAStepResult({ step_id: "QA1", status: "passed" })],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={results} />);

      expect(screen.getByTestId("criterion-status-AC1")).toHaveAttribute("data-status", "passed");
    });

    it("shows X mark for failed criteria when results available", () => {
      const taskQA = createTaskQAResponse({
        acceptance_criteria: [createAcceptanceCriterion({ id: "AC1" })],
        qa_test_steps: [createQATestStep({ id: "QA1", criteria_id: "AC1" })],
      });
      const results = createQAResultsResponse({
        steps: [createQAStepResult({ step_id: "QA1", status: "failed" })],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={results} />);

      expect(screen.getByTestId("criterion-status-AC1")).toHaveAttribute("data-status", "failed");
    });

    it("shows pending icon for criteria without results", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      expect(screen.getByTestId("criterion-status-AC1")).toHaveAttribute("data-status", "pending");
    });
  });

  describe("Test Results tab", () => {
    it("shows empty state when no results", async () => {
      const user = userEvent.setup();
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByText(/no test results/i)).toBeInTheDocument();
    });

    it("shows overall status summary", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        overall_status: "passed",
        passed_steps: 2,
        failed_steps: 0,
        total_steps: 2,
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByTestId("overall-status")).toHaveTextContent(/passed/i);
      expect(screen.getByTestId("results-summary")).toHaveTextContent("2/2");
    });

    it("renders all step results", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        qa_test_steps: [
          createQATestStep({ id: "QA1", description: "Step one" }),
          createQATestStep({ id: "QA2", description: "Step two" }),
        ],
      });
      const results = createQAResultsResponse({
        total_steps: 2,
        steps: [
          createQAStepResult({ step_id: "QA1", status: "passed" }),
          createQAStepResult({ step_id: "QA2", status: "failed" }),
        ],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByText("Step one")).toBeInTheDocument();
      expect(screen.getByText("Step two")).toBeInTheDocument();
    });

    it("shows pass icon for passed steps", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        steps: [createQAStepResult({ step_id: "QA1", status: "passed" })],
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByTestId("step-status-QA1")).toHaveAttribute("data-status", "passed");
    });

    it("shows fail icon for failed steps", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        overall_status: "failed",
        failed_steps: 1,
        passed_steps: 0,
        steps: [createQAStepResult({ step_id: "QA1", status: "failed" })],
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByTestId("step-status-QA1")).toHaveAttribute("data-status", "failed");
    });

    it("shows skipped icon for skipped steps", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        steps: [createQAStepResult({ step_id: "QA1", status: "skipped" })],
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByTestId("step-status-QA1")).toHaveAttribute("data-status", "skipped");
    });

    it("shows screenshot link when step has screenshot", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        steps: [createQAStepResult({ step_id: "QA1", screenshot: "screenshots/qa1.png" })],
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByTestId("step-screenshot-link-QA1")).toBeInTheDocument();
    });
  });

  describe("failure details", () => {
    it("shows expected vs actual for failed steps", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        overall_status: "failed",
        failed_steps: 1,
        passed_steps: 0,
        steps: [
          createQAStepResult({
            step_id: "QA1",
            status: "failed",
            expected: "7 columns visible",
            actual: "5 columns visible",
          }),
        ],
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByTestId("failure-details-QA1")).toBeInTheDocument();
      expect(screen.getByText("7 columns visible")).toBeInTheDocument();
      expect(screen.getByText("5 columns visible")).toBeInTheDocument();
    });

    it("shows error message for failed steps", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        overall_status: "failed",
        failed_steps: 1,
        passed_steps: 0,
        steps: [
          createQAStepResult({
            step_id: "QA1",
            status: "failed",
            error: "Element not found: [data-testid='column-draft']",
          }),
        ],
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.getByTestId("error-message-QA1")).toHaveTextContent("Element not found");
    });

    it("does not show failure details for passed steps", async () => {
      const user = userEvent.setup();
      const results = createQAResultsResponse({
        steps: [createQAStepResult({ step_id: "QA1", status: "passed" })],
      });

      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={results} />);
      await user.click(screen.getByRole("tab", { name: /test results/i }));

      expect(screen.queryByTestId("failure-details-QA1")).not.toBeInTheDocument();
    });
  });

  describe("Screenshots tab", () => {
    it("shows empty state when no screenshots", async () => {
      const user = userEvent.setup();
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse({ screenshots: [] })} results={null} />);

      await user.click(screen.getByRole("tab", { name: /screenshots/i }));

      expect(screen.getByText(/no screenshots/i)).toBeInTheDocument();
    });

    it("renders thumbnail gallery", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        screenshots: ["screenshots/qa1.png", "screenshots/qa2.png"],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);
      await user.click(screen.getByRole("tab", { name: /screenshots/i }));

      expect(screen.getByTestId("screenshot-thumbnail-0")).toBeInTheDocument();
      expect(screen.getByTestId("screenshot-thumbnail-1")).toBeInTheDocument();
    });

    it("opens lightbox when thumbnail is clicked", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        screenshots: ["screenshots/qa1.png"],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);
      await user.click(screen.getByRole("tab", { name: /screenshots/i }));
      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("screenshot-lightbox")).toBeInTheDocument();
    });

    it("closes lightbox when close button is clicked", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        screenshots: ["screenshots/qa1.png"],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);
      await user.click(screen.getByRole("tab", { name: /screenshots/i }));
      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-close"));

      expect(screen.queryByTestId("screenshot-lightbox")).not.toBeInTheDocument();
    });

    it("closes lightbox when escape key is pressed", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        screenshots: ["screenshots/qa1.png"],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);
      await user.click(screen.getByRole("tab", { name: /screenshots/i }));
      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.keyboard("{Escape}");

      expect(screen.queryByTestId("screenshot-lightbox")).not.toBeInTheDocument();
    });

    it("navigates to next screenshot in lightbox", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        screenshots: ["screenshots/qa1.png", "screenshots/qa2.png"],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);
      await user.click(screen.getByRole("tab", { name: /screenshots/i }));
      await user.click(screen.getByTestId("screenshot-thumbnail-0"));
      await user.click(screen.getByTestId("lightbox-next"));

      expect(screen.getByTestId("lightbox-current-index")).toHaveTextContent("2 of 2");
    });

    it("navigates to previous screenshot in lightbox", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        screenshots: ["screenshots/qa1.png", "screenshots/qa2.png"],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);
      await user.click(screen.getByRole("tab", { name: /screenshots/i }));
      await user.click(screen.getByTestId("screenshot-thumbnail-1"));
      await user.click(screen.getByTestId("lightbox-prev"));

      expect(screen.getByTestId("lightbox-current-index")).toHaveTextContent("1 of 2");
    });

    it("shows screenshot filename in lightbox", async () => {
      const user = userEvent.setup();
      const taskQA = createTaskQAResponse({
        screenshots: ["screenshots/qa1-result.png"],
      });

      render(<TaskDetailQAPanel taskQA={taskQA} results={null} />);
      await user.click(screen.getByRole("tab", { name: /screenshots/i }));
      await user.click(screen.getByTestId("screenshot-thumbnail-0"));

      expect(screen.getByTestId("lightbox-filename")).toHaveTextContent("qa1-result.png");
    });
  });

  describe("loading and empty states", () => {
    it("shows loading skeleton when isLoading is true", () => {
      render(<TaskDetailQAPanel taskQA={null} results={null} isLoading />);

      expect(screen.getByTestId("qa-panel-skeleton")).toBeInTheDocument();
    });

    it("shows empty state when taskQA is null", () => {
      render(<TaskDetailQAPanel taskQA={null} results={null} />);

      expect(screen.getByText(/no qa data/i)).toBeInTheDocument();
    });
  });

  describe("action buttons", () => {
    it("shows retry button when results exist and failed", () => {
      const results = createQAResultsResponse({ overall_status: "failed" });

      render(
        <TaskDetailQAPanel
          taskQA={createTaskQAResponse()}
          results={results}
          onRetry={vi.fn()}
          onSkip={vi.fn()}
        />
      );

      expect(screen.getByRole("button", { name: /retry/i })).toBeInTheDocument();
    });

    it("shows skip button when results exist and failed", () => {
      const results = createQAResultsResponse({ overall_status: "failed" });

      render(
        <TaskDetailQAPanel
          taskQA={createTaskQAResponse()}
          results={results}
          onRetry={vi.fn()}
          onSkip={vi.fn()}
        />
      );

      expect(screen.getByRole("button", { name: /skip/i })).toBeInTheDocument();
    });

    it("calls onRetry when retry button is clicked", async () => {
      const user = userEvent.setup();
      const onRetry = vi.fn();
      const results = createQAResultsResponse({ overall_status: "failed" });

      render(
        <TaskDetailQAPanel
          taskQA={createTaskQAResponse()}
          results={results}
          onRetry={onRetry}
          onSkip={vi.fn()}
        />
      );

      await user.click(screen.getByRole("button", { name: /retry/i }));

      expect(onRetry).toHaveBeenCalledTimes(1);
    });

    it("calls onSkip when skip button is clicked", async () => {
      const user = userEvent.setup();
      const onSkip = vi.fn();
      const results = createQAResultsResponse({ overall_status: "failed" });

      render(
        <TaskDetailQAPanel
          taskQA={createTaskQAResponse()}
          results={results}
          onRetry={vi.fn()}
          onSkip={onSkip}
        />
      );

      await user.click(screen.getByRole("button", { name: /skip/i }));

      expect(onSkip).toHaveBeenCalledTimes(1);
    });

    it("disables buttons when actions are in progress", () => {
      const results = createQAResultsResponse({ overall_status: "failed" });

      render(
        <TaskDetailQAPanel
          taskQA={createTaskQAResponse()}
          results={results}
          onRetry={vi.fn()}
          onSkip={vi.fn()}
          isRetrying
          isSkipping
        />
      );

      expect(screen.getByRole("button", { name: /retry/i })).toBeDisabled();
      expect(screen.getByRole("button", { name: /skip/i })).toBeDisabled();
    });

    it("hides action buttons when not failed", () => {
      const results = createQAResultsResponse({ overall_status: "passed" });

      render(
        <TaskDetailQAPanel
          taskQA={createTaskQAResponse()}
          results={results}
          onRetry={vi.fn()}
          onSkip={vi.fn()}
        />
      );

      expect(screen.queryByRole("button", { name: /retry/i })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /skip/i })).not.toBeInTheDocument();
    });
  });

  describe("accessibility", () => {
    it("has proper ARIA roles for tabs", () => {
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      expect(screen.getByRole("tablist")).toBeInTheDocument();
      expect(screen.getAllByRole("tab")).toHaveLength(3);
      expect(screen.getByRole("tabpanel")).toBeInTheDocument();
    });

    it("uses keyboard navigation for tabs", async () => {
      const user = userEvent.setup();
      render(<TaskDetailQAPanel taskQA={createTaskQAResponse()} results={null} />);

      const criteriaTab = screen.getByRole("tab", { name: /acceptance criteria/i });
      criteriaTab.focus();

      await user.keyboard("{ArrowRight}");
      expect(screen.getByRole("tab", { name: /test results/i })).toHaveFocus();

      await user.keyboard("{ArrowRight}");
      expect(screen.getByRole("tab", { name: /screenshots/i })).toHaveFocus();
    });
  });
});
