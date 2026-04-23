import React from "react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MergedTaskDetail } from "./MergedTaskDetail";
import type { ArtifactResponse } from "@/api/artifacts";
import type { PlanBranch } from "@/api/plan-branch.types";
import type { Task } from "@/types/task";
import type { ReviewNoteResponse } from "@/lib/tauri";

const mockPlanBranchState = vi.hoisted((): { current: PlanBranch | null } => ({
  current: null,
}));

const mockPlanArtifactState = vi.hoisted((): { current: ArtifactResponse | null } => ({
  current: {
    id: "artifact-123",
    name: "Plan: Fix graph crash",
    artifact_type: "specification",
    content_type: "inline",
    content:
      "# Fix graph crash when no active plan selected\n\nGuard graph rendering when no plan is selected.",
    created_at: "2026-01-28T12:00:00+00:00",
    created_by: "test",
    version: 3,
    bucket_id: null,
    task_id: null,
    process_id: null,
    derived_from: [],
  },
}));

const mockGitDiffState = vi.hoisted(() => ({
  commits: [] as Array<{
    sha: string;
    shortSha: string;
    message: string;
    author: string;
    date: Date;
  }>,
}));

const mockPlanReviewState = vi.hoisted(() => ({
  tasks: [] as Task[],
  historiesByTaskId: new Map<string, ReviewNoteResponse[]>(),
}));

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: {
      list: vi.fn(async () => ({
        tasks: mockPlanReviewState.tasks,
        total: mockPlanReviewState.tasks.length,
        hasMore: false,
        offset: 0,
      })),
    },
    reviews: {
      getTaskStateHistory: vi.fn(async (taskId: string) =>
        mockPlanReviewState.historiesByTaskId.get(taskId) ?? []
      ),
    },
    artifacts: {
      getArtifact: vi.fn(async () => mockPlanArtifactState.current),
    },
  },
}));

vi.mock("@/hooks/usePlanBranchForTask", () => ({
  usePlanBranchForTask: vi.fn(() => ({ data: mockPlanBranchState.current })),
}));

vi.mock("@/hooks/useGitDiff", () => ({
  useGitDiff: vi.fn(() => ({
    changes: [],
    commits: mockGitDiffState.commits,
    commitFiles: [],
    isLoadingChanges: false,
    isLoadingHistory: false,
    isLoadingCommitFiles: false,
    error: null,
    fetchDiff: vi.fn(),
    fetchCommitFiles: vi.fn(),
    refresh: vi.fn(),
  })),
}));

vi.mock("@/hooks/useReviews", () => ({
  useTaskStateHistory: vi.fn(() => ({ data: [], isLoading: false })),
}));

vi.mock("@/hooks/useTaskStateTransitions", () => ({
  useTaskStateTransitions: vi.fn(() => ({ data: [] })),
}));

vi.mock("@/hooks/useTaskMetrics", () => ({
  useTaskMetrics: vi.fn(() => ({
    data: {
      stepCount: 0,
      completedStepCount: 0,
      reviewCount: 0,
      executionMinutes: 0,
    },
    isLoading: false,
    isError: false,
  })),
}));

vi.mock("@/components/reviews/ReviewDetailModal", () => ({
  ReviewDetailModal: ({ taskId }: { taskId: string }) => (
    <div data-testid="review-detail-modal">Review modal for {taskId}</div>
  ),
}));

function createTestTask(overrides?: Partial<Task>): Task {
  return {
    id: "task-123",
    projectId: "project-456",
    category: "feature",
    title: "Test Task",
    description: "Test description",
    priority: 2,
    internalStatus: "merged",
    needsReviewPoint: false,
    createdAt: "2026-01-28T12:00:00+00:00",
    updatedAt: "2026-01-28T12:00:00+00:00",
    startedAt: null,
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

function createTestPlanBranch(overrides?: Partial<PlanBranch>): PlanBranch {
  return {
    id: "plan-branch-123",
    planArtifactId: "artifact-123",
    sessionId: "session-123",
    projectId: "project-456",
    branchName: "ralphx/ralphx/plan-a3612efd",
    sourceBranch: "main",
    status: "merged",
    mergeTaskId: "task-123",
    createdAt: "2026-01-28T12:00:00+00:00",
    mergedAt: "2026-01-28T12:30:00+00:00",
    prNumber: 68,
    prUrl: "https://github.com/aigentive/ralphx/pull/68",
    prDraft: false,
    prPushStatus: "pushed",
    prStatus: "Open",
    prPollingActive: false,
    prEligible: true,
    mergeCommitSha: null,
    baseBranchOverride: null,
    ...overrides,
  };
}

function createReviewNote(overrides?: Partial<ReviewNoteResponse>): ReviewNoteResponse {
  return {
    id: "review-note-1",
    task_id: "plan-task-1",
    reviewer: "ai",
    outcome: "approved",
    summary: "Plan implementation review passed",
    notes: "No follow-up work needed.",
    issues: [],
    followup_session_id: null,
    created_at: "2026-01-28T12:20:00+00:00",
    ...overrides,
  };
}

function TestWrapper({ children }: { children: React.ReactNode }) {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

function renderWithProviders(ui: React.ReactElement) {
  return render(ui, { wrapper: TestWrapper });
}

describe("MergedTaskDetail", () => {
  beforeEach(() => {
    mockPlanBranchState.current = null;
    mockPlanArtifactState.current = {
      id: "artifact-123",
      name: "Plan: Fix graph crash",
      artifact_type: "specification",
      content_type: "inline",
      content:
        "# Fix graph crash when no active plan selected\n\nGuard graph rendering when no plan is selected.",
      created_at: "2026-01-28T12:00:00+00:00",
      created_by: "test",
      version: 3,
      bucket_id: null,
      task_id: null,
      process_id: null,
      derived_from: [],
    };
    mockGitDiffState.commits = [];
    const planTask = createTestTask({
      id: "plan-task-1",
      title: "Fix graph crash",
      category: "feature",
      internalStatus: "merged",
    });
    mockPlanReviewState.tasks = [planTask];
    mockPlanReviewState.historiesByTaskId = new Map([
      [planTask.id, [createReviewNote({ task_id: planTask.id })]],
    ]);
  });

  it("uses PR plan-branch context for merged plan-merge tasks with stale PR status", async () => {
    mockPlanBranchState.current = createTestPlanBranch({
      mergeCommitSha: "abc123456789",
    });
    mockGitDiffState.commits = [
      {
        sha: "abc123456789",
        shortSha: "abc1234",
        message: "Merge pull request #68",
        author: "GitHub",
        date: new Date("2026-01-28T12:30:00+00:00"),
      },
    ];
    const task = createTestTask({
      category: "plan_merge",
      taskBranch: null,
      mergeCommitSha: null,
    });

    renderWithProviders(<MergedTaskDetail task={task} />);

    expect(await screen.findByText("Fix graph crash when no active plan selected")).toBeInTheDocument();
    expect(screen.getByText("Guard graph rendering when no plan is selected.")).toBeInTheDocument();
    expect(screen.getByText("Merged via PR #68")).toBeInTheDocument();
    expect(screen.queryByTestId("task-metrics-section")).not.toBeInTheDocument();
    expect(screen.getByText("Merge Commit")).toBeInTheDocument();
    expect(screen.queryByText("unknown")).not.toBeInTheDocument();
    expect(screen.getAllByText("abc1234").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Merge pull request #68")).toBeInTheDocument();
    expect(screen.getByText("Code Review")).toBeInTheDocument();
    expect(screen.getByText("Review Diff")).toBeInTheDocument();
    expect(await screen.findByText("Plan implementation review passed")).toBeInTheDocument();
    expect(screen.getByText("Fix graph crash")).toBeInTheDocument();
    expect(screen.queryByText("No review history available")).not.toBeInTheDocument();
    expect(screen.queryByText("No internal plan review records available")).not.toBeInTheDocument();
  });

  it("omits merge details instead of showing unknown when a merged plan merge has no SHA", async () => {
    mockPlanBranchState.current = createTestPlanBranch();
    const task = createTestTask({
      category: "plan_merge",
      taskBranch: null,
      mergeCommitSha: null,
    });

    renderWithProviders(<MergedTaskDetail task={task} />);

    expect(await screen.findByTestId("plan-merge-context-section")).toBeInTheDocument();
    expect(screen.getByText("Merged via PR #68")).toBeInTheDocument();
    expect(screen.queryByTestId("merge-info-section")).not.toBeInTheDocument();
    expect(screen.queryByText("unknown")).not.toBeInTheDocument();
  });
});
