import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PlanGroupSettings } from "./PlanGroupSettings";
import type { PlanBranch } from "@/api/plan-branch.types";

// Mock tauri API
const mockEnable = vi.fn();
const mockDisable = vi.fn();
vi.mock("@/lib/tauri", () => ({
  api: {
    planBranches: {
      enable: (...args: unknown[]) => mockEnable(...args),
      disable: (...args: unknown[]) => mockDisable(...args),
    },
  },
}));

// Mock git branches API
const mockGetGitBranches = vi.fn();
vi.mock("@/api/projects", () => ({
  getGitBranches: (...args: unknown[]) => mockGetGitBranches(...args),
}));

const defaultProps = {
  planArtifactId: "plan-123",
  sessionId: "session-456",
  projectId: "project-789",
  planBranch: null,
  hasMergedTasks: false,
  onBranchChange: vi.fn(),
  workingDirectory: "/some/path",
  baseBranch: "main",
};

function renderSettings(props = {}) {
  return render(<PlanGroupSettings {...defaultProps} {...props} />);
}

describe("PlanGroupSettings", () => {
  beforeEach(() => {
    mockGetGitBranches.mockResolvedValue([]);
    mockEnable.mockResolvedValue(undefined);
    mockDisable.mockResolvedValue(undefined);
  });

  describe("branch selector input type", () => {
    it("renders text input (not Select) when branch selector is shown", async () => {
      renderSettings();
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const input = screen.getByTestId("plan-group-branch-input");
      expect(input).toBeInTheDocument();
      expect(input).toHaveAttribute("type", "text");
      expect(input).toHaveAttribute("list", "plan-group-branch-datalist");
    });

    it("renders datalist element alongside input", async () => {
      mockGetGitBranches.mockResolvedValue(["main", "develop"]);
      renderSettings();
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      await waitFor(() => {
        const datalist = document.getElementById("plan-group-branch-datalist");
        expect(datalist).toBeInTheDocument();
        const options = datalist!.querySelectorAll("option");
        expect(options).toHaveLength(2);
        expect(options[0]).toHaveValue("main");
        expect(options[1]).toHaveValue("develop");
      });
    });

    it("renders text input even when branches are available (no conditional Select)", async () => {
      mockGetGitBranches.mockResolvedValue(["main", "feature/foo"]);
      renderSettings();
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      await waitFor(() => {
        const input = screen.getByTestId("plan-group-branch-input");
        expect(input.tagName).toBe("INPUT");
        expect(input).toHaveAttribute("type", "text");
      });
      // No native <select> element (confirming no Select component was rendered)
      expect(document.querySelector("select")).not.toBeInTheDocument();
    });
  });

  describe("input typing", () => {
    it("updates value on user typing", async () => {
      renderSettings({ baseBranch: "" });
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const input = screen.getByTestId("plan-group-branch-input") as HTMLInputElement;
      fireEvent.change(input, { target: { value: "feature/my-branch" } });
      expect(input.value).toBe("feature/my-branch");
    });

    it("initializes with baseBranch value", async () => {
      renderSettings({ baseBranch: "develop" });
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const input = screen.getByTestId("plan-group-branch-input") as HTMLInputElement;
      expect(input.value).toBe("develop");
    });
  });

  describe("Enable button disabled with trim()", () => {
    it("is disabled when input is empty", async () => {
      renderSettings({ baseBranch: "" });
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const input = screen.getByTestId("plan-group-branch-input");
      fireEvent.change(input, { target: { value: "" } });

      const enableBtn = screen.getByRole("button", { name: /enable/i });
      expect(enableBtn).toBeDisabled();
    });

    it("is disabled when input contains only whitespace", async () => {
      renderSettings({ baseBranch: "" });
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const input = screen.getByTestId("plan-group-branch-input");
      fireEvent.change(input, { target: { value: "   " } });

      const enableBtn = screen.getByRole("button", { name: /enable/i });
      expect(enableBtn).toBeDisabled();
    });

    it("is enabled when input has non-whitespace content", async () => {
      renderSettings({ baseBranch: "main" });
      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const enableBtn = screen.getByRole("button", { name: /enable/i });
      expect(enableBtn).not.toBeDisabled();
    });
  });

  describe("handleConfirmEnable — error handling", () => {
    it("keeps branch selector visible on API error (error-retry flow)", async () => {
      mockEnable.mockRejectedValue(new Error("Branch not found"));
      renderSettings({ baseBranch: "main" });

      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const enableBtn = screen.getByRole("button", { name: /enable/i });
      await userEvent.click(enableBtn);

      await waitFor(() => {
        // Selector stays visible for retry
        expect(screen.getByTestId("plan-group-branch-input")).toBeInTheDocument();
        // Error message shown
        expect(screen.getByText(/branch not found/i)).toBeInTheDocument();
      });
    });

    it("hides branch selector on success", async () => {
      mockEnable.mockResolvedValue(undefined);
      renderSettings({ baseBranch: "main" });

      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const enableBtn = screen.getByRole("button", { name: /enable/i });
      await userEvent.click(enableBtn);

      await waitFor(() => {
        expect(screen.queryByTestId("plan-group-branch-input")).not.toBeInTheDocument();
      });
    });

    it("passes trimmed branch to enable API", async () => {
      mockEnable.mockResolvedValue(undefined);
      renderSettings({ baseBranch: "" });

      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const input = screen.getByTestId("plan-group-branch-input");
      fireEvent.change(input, { target: { value: "  main  " } });

      const enableBtn = screen.getByRole("button", { name: /enable/i });
      await userEvent.click(enableBtn);

      await waitFor(() => {
        expect(mockEnable).toHaveBeenCalledWith(
          expect.objectContaining({ baseBranchOverride: "main" })
        );
      });
    });

    it("calls onBranchChange in finally regardless of error", async () => {
      const onBranchChange = vi.fn();
      mockEnable.mockRejectedValue(new Error("Fail"));
      renderSettings({ baseBranch: "main", onBranchChange });

      const toggle = screen.getByRole("switch");
      await userEvent.click(toggle);

      const enableBtn = screen.getByRole("button", { name: /enable/i });
      await userEvent.click(enableBtn);

      await waitFor(() => {
        expect(onBranchChange).toHaveBeenCalled();
      });
    });
  });

  describe("planBranch prop display", () => {
    it("shows branch info when planBranch is provided", () => {
      const planBranch: PlanBranch = {
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
      renderSettings({ planBranch });

      expect(screen.getByText("feature/my-plan")).toBeInTheDocument();
      expect(screen.getByText("Active")).toBeInTheDocument();
    });
  });
});
