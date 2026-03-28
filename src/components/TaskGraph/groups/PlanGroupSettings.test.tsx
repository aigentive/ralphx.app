import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanGroupSettings } from "./PlanGroupSettings";
import type { PlanBranch } from "@/api/plan-branch.types";

const mockPlanBranch: PlanBranch = {
  id: "pb-1",
  planArtifactId: "plan-123",
  sessionId: "session-456",
  projectId: "project-789",
  branchName: "feature/my-plan",
  sourceBranch: "main",
  status: "active",
  baseBranchOverride: null,
  mergeTaskId: null,
  createdAt: "2026-01-01T00:00:00Z",
  mergedAt: null,
  prNumber: null,
  prUrl: null,
  prDraft: null,
  prPushStatus: null,
  prStatus: null,
  prPollingActive: false,
  prEligible: false,
};

describe("PlanGroupSettings", () => {
  describe("header", () => {
    it("renders Feature Branch header", () => {
      render(<PlanGroupSettings planBranch={null} />);
      expect(screen.getByText("Feature Branch")).toBeInTheDocument();
    });
  });

  describe("no planBranch", () => {
    it("does not show branch detail section when planBranch is null", () => {
      render(<PlanGroupSettings planBranch={null} />);
      expect(screen.queryByText("Active")).not.toBeInTheDocument();
      expect(screen.queryByText("Merged")).not.toBeInTheDocument();
      expect(screen.queryByText("View merge task")).not.toBeInTheDocument();
    });
  });

  describe("planBranch display", () => {
    it("shows branch name", () => {
      render(<PlanGroupSettings planBranch={mockPlanBranch} />);
      expect(screen.getByText("feature/my-plan")).toBeInTheDocument();
    });

    it("shows Active status", () => {
      render(<PlanGroupSettings planBranch={mockPlanBranch} />);
      expect(screen.getByText("Active")).toBeInTheDocument();
    });

    it("shows source branch", () => {
      render(<PlanGroupSettings planBranch={mockPlanBranch} />);
      expect(screen.getByText("main")).toBeInTheDocument();
    });

    it("shows Merged status", () => {
      render(<PlanGroupSettings planBranch={{ ...mockPlanBranch, status: "merged" }} />);
      expect(screen.getByText("Merged")).toBeInTheDocument();
    });

    it("shows Abandoned status", () => {
      render(<PlanGroupSettings planBranch={{ ...mockPlanBranch, status: "abandoned" }} />);
      expect(screen.getByText("Abandoned")).toBeInTheDocument();
    });

    it("shows merge target when baseBranchOverride is set", () => {
      render(
        <PlanGroupSettings planBranch={{ ...mockPlanBranch, baseBranchOverride: "develop" }} />
      );
      expect(screen.getByText("develop")).toBeInTheDocument();
    });

    it("does not show merge target row when baseBranchOverride is null", () => {
      render(<PlanGroupSettings planBranch={mockPlanBranch} />);
      expect(screen.queryByText("Merge Target")).not.toBeInTheDocument();
    });
  });

  describe("merge task link", () => {
    it("shows View merge task link when mergeTaskId is set", () => {
      const onNavigateToMergeTask = vi.fn();
      render(
        <PlanGroupSettings
          planBranch={{ ...mockPlanBranch, mergeTaskId: "task-999" }}
          onNavigateToMergeTask={onNavigateToMergeTask}
        />
      );
      expect(screen.getByText("View merge task")).toBeInTheDocument();
    });

    it("calls onNavigateToMergeTask with task id when link clicked", async () => {
      const onNavigateToMergeTask = vi.fn();
      const user = userEvent.setup();
      render(
        <PlanGroupSettings
          planBranch={{ ...mockPlanBranch, mergeTaskId: "task-999" }}
          onNavigateToMergeTask={onNavigateToMergeTask}
        />
      );

      await user.click(screen.getByText("View merge task"));
      expect(onNavigateToMergeTask).toHaveBeenCalledWith("task-999");
    });

    it("does not show link when mergeTaskId is null", () => {
      render(<PlanGroupSettings planBranch={mockPlanBranch} />);
      expect(screen.queryByText("View merge task")).not.toBeInTheDocument();
    });

    it("does not show link when onNavigateToMergeTask is not provided", () => {
      render(
        <PlanGroupSettings planBranch={{ ...mockPlanBranch, mergeTaskId: "task-999" }} />
      );
      expect(screen.queryByText("View merge task")).not.toBeInTheDocument();
    });
  });
});
