/**
 * End-to-end QA UI Flow Integration Tests
 *
 * Tests the complete QA UI flow:
 * - TaskQABadge displays on task cards when needsQA=true
 * - Badge updates through QA states (pending -> preparing -> ready -> testing -> passed/failed)
 * - TaskDetailQAPanel renders with QA data
 * - QA results display correctly
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, waitFor } from "@testing-library/react";
import { DndContext } from "@dnd-kit/core";
import { createMockTask } from "@/test/mock-data";
import { TaskCard } from "@/components/tasks/TaskBoard/TaskCard";
import { TaskDetailQAPanel } from "@/components/qa/TaskDetailQAPanel";
import { useQAStore } from "@/stores/qaStore";
import type { QAPrepStatus } from "@/types/qa-config";
import type { QAOverallStatus } from "@/types/qa";
import type { TaskQAResponse, QAResultsResponse } from "@/lib/tauri";

// Mock Tauri event listener
const mockListeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, callback: (event: { payload: unknown }) => void) => {
    mockListeners.set(eventName, callback);
    return Promise.resolve(() => {
      mockListeners.delete(eventName);
    });
  }),
}));

// Wrapper component for dnd-kit context
function DndWrapper({ children }: { children: React.ReactNode }) {
  return <DndContext>{children}</DndContext>;
}

describe("End-to-end QA UI Flow", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListeners.clear();
    // Reset store state
    useQAStore.setState({
      settings: {
        qa_enabled: true,
        auto_qa_for_ui_tasks: true,
        auto_qa_for_api_tasks: false,
        qa_prep_enabled: true,
        browser_testing_enabled: true,
        browser_testing_url: "http://localhost:1420",
      },
      settingsLoaded: true,
      taskQA: {},
      isLoadingSettings: false,
      loadingTasks: new Set(),
      error: null,
    });
  });

  afterEach(() => {
    mockListeners.clear();
  });

  describe("TaskQABadge on TaskCard", () => {
    it("should show QA badge when needsQA is true", () => {
      const task = createMockTask({ id: "task-qa-1", title: "QA Task" });
      render(<TaskCard task={task} needsQA />, { wrapper: DndWrapper });

      expect(screen.getByTestId("task-qa-badge")).toBeInTheDocument();
    });

    it("should not show QA badge when needsQA is false", () => {
      const task = createMockTask({ id: "task-no-qa", title: "No QA Task" });
      render(<TaskCard task={task} needsQA={false} />, { wrapper: DndWrapper });

      expect(screen.queryByTestId("task-qa-badge")).not.toBeInTheDocument();
    });

    it("should show pending status initially", () => {
      const task = createMockTask({ id: "task-pending", title: "Pending Task" });
      render(<TaskCard task={task} needsQA />, { wrapper: DndWrapper });

      expect(screen.getByText("QA Pending")).toBeInTheDocument();
    });

    it("should show preparing status when prep is running", () => {
      const task = createMockTask({ id: "task-prep", title: "Prep Task" });
      const prepStatus: QAPrepStatus = "running";
      render(<TaskCard task={task} needsQA prepStatus={prepStatus} />, {
        wrapper: DndWrapper,
      });

      expect(screen.getByText("Preparing")).toBeInTheDocument();
    });

    it("should show ready status when prep is completed", () => {
      const task = createMockTask({ id: "task-ready", title: "Ready Task" });
      const prepStatus: QAPrepStatus = "completed";
      render(<TaskCard task={task} needsQA prepStatus={prepStatus} />, {
        wrapper: DndWrapper,
      });

      expect(screen.getByText("QA Ready")).toBeInTheDocument();
    });

    it("should show testing status when test is running", () => {
      const task = createMockTask({ id: "task-testing", title: "Testing Task" });
      const testStatus: QAOverallStatus = "running";
      render(<TaskCard task={task} needsQA testStatus={testStatus} />, {
        wrapper: DndWrapper,
      });

      expect(screen.getByText("Testing")).toBeInTheDocument();
    });

    it("should show passed status when test is passed", () => {
      const task = createMockTask({ id: "task-passed", title: "Passed Task" });
      const testStatus: QAOverallStatus = "passed";
      render(<TaskCard task={task} needsQA testStatus={testStatus} />, {
        wrapper: DndWrapper,
      });

      expect(screen.getByText("Passed")).toBeInTheDocument();
    });

    it("should show failed status when test is failed", () => {
      const task = createMockTask({ id: "task-failed", title: "Failed Task" });
      const testStatus: QAOverallStatus = "failed";
      render(<TaskCard task={task} needsQA testStatus={testStatus} />, {
        wrapper: DndWrapper,
      });

      expect(screen.getByText("Failed")).toBeInTheDocument();
    });
  });

  describe("Badge updates through QA states", () => {
    it("should update badge as status changes (pending -> preparing -> ready)", () => {
      const task = createMockTask({ id: "task-flow", title: "Flow Task" });

      // Initial: pending
      const { rerender } = render(<TaskCard task={task} needsQA />, {
        wrapper: DndWrapper,
      });
      expect(screen.getByText("QA Pending")).toBeInTheDocument();

      // Update: preparing
      rerender(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="running" />
        </DndWrapper>
      );
      expect(screen.getByText("Preparing")).toBeInTheDocument();

      // Update: ready
      rerender(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="completed" />
        </DndWrapper>
      );
      expect(screen.getByText("QA Ready")).toBeInTheDocument();
    });

    it("should update badge through testing states (ready -> testing -> passed)", () => {
      const task = createMockTask({ id: "task-test-flow", title: "Test Flow Task" });

      // Initial: ready (prep completed)
      const { rerender } = render(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="completed" />
        </DndWrapper>
      );
      expect(screen.getByText("QA Ready")).toBeInTheDocument();

      // Update: testing
      rerender(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="completed" testStatus="running" />
        </DndWrapper>
      );
      expect(screen.getByText("Testing")).toBeInTheDocument();

      // Update: passed
      rerender(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="completed" testStatus="passed" />
        </DndWrapper>
      );
      expect(screen.getByText("Passed")).toBeInTheDocument();
    });

    it("should update badge through failure states (ready -> testing -> failed)", () => {
      const task = createMockTask({ id: "task-fail-flow", title: "Fail Flow Task" });

      // Initial: ready
      const { rerender } = render(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="completed" />
        </DndWrapper>
      );
      expect(screen.getByText("QA Ready")).toBeInTheDocument();

      // Update: testing
      rerender(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="completed" testStatus="running" />
        </DndWrapper>
      );
      expect(screen.getByText("Testing")).toBeInTheDocument();

      // Update: failed
      rerender(
        <DndWrapper>
          <TaskCard task={task} needsQA prepStatus="completed" testStatus="failed" />
        </DndWrapper>
      );
      expect(screen.getByText("Failed")).toBeInTheDocument();
    });
  });

  describe("TaskDetailQAPanel rendering", () => {
    const mockTaskQA: TaskQAResponse = {
      id: "qa-123",
      task_id: "task-detail-1",
      acceptance_criteria: [
        {
          id: "ac-1",
          description: "Login button should be visible",
          criteria_type: "functional",
          testable: true,
        },
        {
          id: "ac-2",
          description: "Error message shows on invalid input",
          criteria_type: "error_handling",
          testable: true,
        },
      ],
      screenshots: ["/screenshots/login-page.png"],
      created_at: "2026-01-24T12:00:00Z",
    };

    const mockResults: QAResultsResponse = {
      task_id: "task-detail-1",
      overall_status: "failed",
      passed_steps: 1,
      failed_steps: 1,
      total_steps: 2,
      steps: [
        {
          step_id: "step-1",
          status: "passed",
        },
        {
          step_id: "step-2",
          status: "failed",
          error: "Timeout waiting for error message",
        },
      ],
    };

    it("should render QA panel with acceptance criteria", () => {
      render(<TaskDetailQAPanel taskQA={mockTaskQA} results={null} />);

      // Verify acceptance criteria tab is present
      expect(screen.getByRole("tab", { name: /criteria/i })).toBeInTheDocument();
    });

    it("should display acceptance criteria list", () => {
      render(<TaskDetailQAPanel taskQA={mockTaskQA} results={null} />);

      expect(screen.getByText("Login button should be visible")).toBeInTheDocument();
      expect(screen.getByText("Error message shows on invalid input")).toBeInTheDocument();
    });

    it("should show test results tab", () => {
      render(<TaskDetailQAPanel taskQA={mockTaskQA} results={mockResults} />);

      const testResultsTab = screen.getByRole("tab", { name: /results/i });
      expect(testResultsTab).toBeInTheDocument();
    });

    it("should show screenshots tab when screenshots exist", () => {
      render(<TaskDetailQAPanel taskQA={mockTaskQA} results={null} />);

      const screenshotsTab = screen.getByRole("tab", { name: /screenshots/i });
      expect(screenshotsTab).toBeInTheDocument();
    });

    it("should display test result summary when results exist", async () => {
      render(<TaskDetailQAPanel taskQA={mockTaskQA} results={mockResults} />);

      // Click on test results tab
      const testResultsTab = screen.getByRole("tab", { name: /results/i });
      await act(async () => {
        testResultsTab.click();
      });

      // Verify the summary is shown (passed_steps/total_steps format)
      expect(screen.getByTestId("results-summary")).toHaveTextContent("1/2");
    });
  });

  describe("Loading and empty states", () => {
    it("should show empty state when no QA data", () => {
      render(<TaskDetailQAPanel taskQA={null} results={null} />);

      expect(screen.getByText(/No QA data available/i)).toBeInTheDocument();
    });

    it("should show empty state for no acceptance criteria", () => {
      const emptyQA: TaskQAResponse = {
        id: "qa-empty",
        task_id: "task-empty",
        acceptance_criteria: [],
        screenshots: [],
        created_at: "2026-01-24T12:00:00Z",
      };

      render(<TaskDetailQAPanel taskQA={emptyQA} results={null} />);

      expect(screen.getByText(/No acceptance criteria defined/i)).toBeInTheDocument();
    });

    it("should show empty state for no test results", async () => {
      const qaWithCriteria: TaskQAResponse = {
        id: "qa-no-tests",
        task_id: "task-no-tests",
        acceptance_criteria: [
          {
            id: "ac-1",
            description: "Test item",
            criteria_type: "functional",
            testable: true,
          },
        ],
        screenshots: [],
        created_at: "2026-01-24T12:00:00Z",
      };

      render(<TaskDetailQAPanel taskQA={qaWithCriteria} results={null} />);

      // Switch to test results tab
      const testResultsTab = screen.getByRole("tab", { name: /results/i });
      await act(async () => {
        testResultsTab.click();
      });

      expect(screen.getByText(/No test results available yet/i)).toBeInTheDocument();
    });
  });
});
