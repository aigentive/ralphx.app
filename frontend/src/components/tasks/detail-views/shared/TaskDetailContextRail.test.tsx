import React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TaskDetailContextProvider } from "./TaskDetailContextProvider";
import { TwoColumnLayout } from "./TwoColumnLayout";
import type { PlanBranch } from "@/api/plan-branch.types";
import type { Task } from "@/types/task";
import type { TaskContext } from "@/types/task-context";

const mockTaskContextApi = vi.hoisted(() => ({
  getTaskContext: vi.fn(),
}));

const mockPlanBranchApi = vi.hoisted(() => ({
  getByProject: vi.fn(),
}));

vi.mock("@/api/task-context", () => ({
  taskContextApi: mockTaskContextApi,
}));

vi.mock("@/api/plan-branch", () => ({
  planBranchApi: mockPlanBranchApi,
}));

function createTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task",
    description: "Implement the selected plan item.",
    priority: 2,
    internalStatus: "merged",
    needsReviewPoint: false,
    ideationSessionId: "session-123",
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:15:00+00:00",
    startedAt: "2026-01-28T12:05:00+00:00",
    completedAt: "2026-01-28T12:30:00+00:00",
    archivedAt: null,
    blockedReason: null,
    taskBranch: "ralphx/ralphx/task-123",
    worktreePath: null,
    mergeCommitSha: "abc123456789",
    metadata: null,
    ...overrides,
  };
}

function createTaskContext(overrides?: Partial<TaskContext>): TaskContext {
  return {
    task: {
      id: "task-123",
      project_id: "project-456",
      category: "feature",
      title: "Test Task",
      description: "Implement the selected plan item.",
      priority: 2,
      internal_status: "merged",
      needs_review_point: false,
      ideation_session_id: "session-123",
      created_at: "2026-01-28T12:00:00+00:00",
      updated_at: "2026-01-28T12:15:00+00:00",
      started_at: "2026-01-28T12:05:00+00:00",
      completed_at: "2026-01-28T12:30:00+00:00",
      archived_at: null,
      blocked_reason: null,
      task_branch: "ralphx/ralphx/task-123",
      worktree_path: null,
      merge_commit_sha: "abc123456789",
      metadata: null,
    },
    sourceProposal: {
      id: "proposal-123",
      title: "Fix graph crash",
      description: "The graph should not crash without an active plan.",
      acceptanceCriteria: [],
      implementationNotes: null,
      planVersionAtCreation: 4,
    },
    planArtifact: {
      id: "artifact-123",
      title: "Fix graph crash when no active plan selected",
      artifactType: "specification",
      currentVersion: 4,
      contentPreview: "Guard graph rendering when no plan is selected.",
    },
    relatedArtifacts: [],
    contextHints: [],
    ...overrides,
  };
}

function createPlanBranch(overrides?: Partial<PlanBranch>): PlanBranch {
  return {
    id: "plan-branch-123",
    planArtifactId: "artifact-123",
    sessionId: "session-123",
    projectId: "project-456",
    branchName: "ralphx/ralphx/plan-a3612efd",
    sourceBranch: "main",
    status: "merged",
    mergeTaskId: "merge-task-123",
    createdAt: "2026-01-28T12:00:00+00:00",
    mergedAt: "2026-01-28T12:30:00+00:00",
    prNumber: 68,
    prUrl: "https://github.com/aigentive/ralphx/pull/68",
    prDraft: false,
    prPushStatus: "pushed",
    prStatus: null,
    prPollingActive: false,
    prEligible: true,
    mergeCommitSha: "abc123456789",
    baseBranchOverride: null,
    ...overrides,
  };
}

function renderRail(task: Task, viewMode: React.ComponentProps<typeof TaskDetailContextProvider>["viewMode"]) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <TaskDetailContextProvider task={task} viewMode={viewMode}>
        <TwoColumnLayout description={task.description}>
          <div>Main detail content</div>
        </TwoColumnLayout>
      </TaskDetailContextProvider>
    </QueryClientProvider>
  );
}

describe("TaskDetailContextRail", () => {
  beforeEach(() => {
    mockTaskContextApi.getTaskContext.mockResolvedValue(createTaskContext());
    mockPlanBranchApi.getByProject.mockResolvedValue([createPlanBranch()]);
  });

  it("shows description, plan, branch, PR, and merge context in the common rail", async () => {
    renderRail(createTask({
      id: "merge-task-123",
      category: "plan_merge",
      taskBranch: null,
    }), { kind: "current" });

    expect(screen.getByText("Main detail content")).toBeInTheDocument();
    expect(screen.getByText("Implement the selected plan item.")).toBeInTheDocument();
    expect(await screen.findByText("Fix graph crash when no active plan selected")).toBeInTheDocument();
    expect(screen.getByText("Guard graph rendering when no plan is selected.")).toBeInTheDocument();
    expect(screen.getByText("Source Proposal")).toBeInTheDocument();
    expect(screen.getByText("Fix graph crash")).toBeInTheDocument();
    expect(screen.getByText("plan-a3612efd")).toBeInTheDocument();
    expect(screen.getAllByText("main").length).toBeGreaterThan(0);
    expect(screen.getByText("PR #68")).toBeInTheDocument();
    expect(screen.getByText("Merged")).toBeInTheDocument();
    expect(screen.getByText("abc1234")).toBeInTheDocument();
  });

  it("labels historical views as latest context instead of pretending to show a snapshot", async () => {
    renderRail(createTask(), {
      kind: "historical",
      status: "waiting_on_pr",
      timestamp: "2026-01-28T12:20:00+00:00",
    });

    expect(await screen.findByText("Historical State")).toBeInTheDocument();
    expect(screen.getByText("Waiting on PR")).toBeInTheDocument();
    expect(screen.getByText("Plan, branch, and PR values show the latest task context.")).toBeInTheDocument();
  });
});
