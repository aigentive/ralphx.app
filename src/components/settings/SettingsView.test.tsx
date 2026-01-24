/**
 * SettingsView component tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SettingsView } from "./SettingsView";
import { DEFAULT_PROJECT_SETTINGS } from "@/types/settings";

describe("SettingsView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("Rendering", () => {
    it("renders with default settings", () => {
      render(<SettingsView />);
      expect(screen.getByTestId("settings-view")).toBeInTheDocument();
      expect(screen.getByText("Settings")).toBeInTheDocument();
      expect(screen.getByText("Configure project behavior")).toBeInTheDocument();
    });

    it("renders loading skeleton when isLoading is true", () => {
      render(<SettingsView isLoading={true} />);
      expect(screen.getByTestId("settings-skeleton")).toBeInTheDocument();
    });

    it("renders saving indicator when isSaving is true", () => {
      render(<SettingsView isSaving={true} />);
      expect(screen.getByText("Saving...")).toBeInTheDocument();
    });

    it("renders error message when error is provided", () => {
      const errorMessage = "Failed to save settings";
      render(<SettingsView error={errorMessage} />);
      expect(screen.getByText(errorMessage)).toBeInTheDocument();
    });

    it("renders all four sections", () => {
      render(<SettingsView />);
      expect(screen.getByText("Execution")).toBeInTheDocument();
      expect(screen.getByText("Model")).toBeInTheDocument();
      expect(screen.getByText("Review")).toBeInTheDocument();
      expect(screen.getByText("Supervisor")).toBeInTheDocument();
    });
  });

  describe("Execution Section", () => {
    it("renders all execution settings", () => {
      render(<SettingsView />);
      expect(screen.getByText("Max Concurrent Tasks")).toBeInTheDocument();
      expect(screen.getByText("Auto Commit")).toBeInTheDocument();
      expect(screen.getByText("Pause on Failure")).toBeInTheDocument();
      expect(screen.getByText("Review Before Destructive")).toBeInTheDocument();
    });

    it("displays default max concurrent tasks value", () => {
      render(<SettingsView />);
      const input = screen.getByTestId("max-concurrent-tasks");
      expect(input).toHaveValue(DEFAULT_PROJECT_SETTINGS.execution.max_concurrent_tasks);
    });

    it("toggles auto commit setting", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      const toggle = screen.getByTestId("auto-commit");
      expect(toggle).toHaveAttribute("aria-checked", "true");

      await user.click(toggle);

      expect(onChange).toHaveBeenCalledTimes(1);
      const calledWith = onChange.mock.calls[0][0];
      expect(calledWith.execution.auto_commit).toBe(false);
    });

    it("updates max concurrent tasks", () => {
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      const input = screen.getByTestId("max-concurrent-tasks");
      fireEvent.change(input, { target: { value: "5" } });

      expect(onChange).toHaveBeenCalledTimes(1);
      const calledWith = onChange.mock.calls[0][0];
      expect(calledWith.execution.max_concurrent_tasks).toBe(5);
    });
  });

  describe("Model Section", () => {
    it("renders all model settings", () => {
      render(<SettingsView />);
      expect(screen.getByText("Default Model")).toBeInTheDocument();
      expect(screen.getByText("Allow Opus Upgrade")).toBeInTheDocument();
    });

    it("displays model dropdown with options", () => {
      render(<SettingsView />);
      const select = screen.getByTestId("model-selection");
      expect(select).toHaveValue("sonnet");
      expect(within(select).getByText("Claude Haiku 4.5")).toBeInTheDocument();
      expect(within(select).getByText("Claude Sonnet 4.5")).toBeInTheDocument();
      expect(within(select).getByText("Claude Opus 4.5")).toBeInTheDocument();
    });

    it("changes model selection", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      const select = screen.getByTestId("model-selection");
      await user.selectOptions(select, "opus");

      expect(onChange).toHaveBeenCalled();
      const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1][0];
      expect(lastCall.model.model).toBe("opus");
    });
  });

  describe("Review Section", () => {
    it("renders all review settings", () => {
      render(<SettingsView />);
      expect(screen.getByText("Enable AI Review")).toBeInTheDocument();
      expect(screen.getByText("Auto Create Fix Tasks")).toBeInTheDocument();
      expect(screen.getByText("Require Fix Approval")).toBeInTheDocument();
      expect(screen.getByText("Require Human Review")).toBeInTheDocument();
      expect(screen.getByText("Max Fix Attempts")).toBeInTheDocument();
    });

    it("disables sub-settings when AI review is disabled", async () => {
      const user = userEvent.setup();
      render(<SettingsView />);

      // Disable AI review
      const aiReviewToggle = screen.getByTestId("ai-review-enabled");
      await user.click(aiReviewToggle);

      // Check sub-settings are disabled
      expect(screen.getByTestId("ai-review-auto-fix")).toHaveAttribute("aria-disabled", "true");
      expect(screen.getByTestId("require-fix-approval")).toHaveAttribute("aria-disabled", "true");
      expect(screen.getByTestId("require-human-review")).toHaveAttribute("aria-disabled", "true");
      expect(screen.getByTestId("max-fix-attempts")).toBeDisabled();
    });
  });

  describe("Supervisor Section", () => {
    it("renders all supervisor settings", () => {
      render(<SettingsView />);
      expect(screen.getByText("Enable Supervisor")).toBeInTheDocument();
      expect(screen.getByText("Loop Threshold")).toBeInTheDocument();
      expect(screen.getByText("Stuck Timeout")).toBeInTheDocument();
    });

    it("displays default supervisor values", () => {
      render(<SettingsView />);
      expect(screen.getByTestId("loop-threshold")).toHaveValue(
        DEFAULT_PROJECT_SETTINGS.supervisor.loop_threshold
      );
      expect(screen.getByTestId("stuck-timeout")).toHaveValue(
        DEFAULT_PROJECT_SETTINGS.supervisor.stuck_timeout
      );
    });

    it("disables sub-settings when supervisor is disabled", async () => {
      const user = userEvent.setup();
      render(<SettingsView />);

      // Disable supervisor
      const supervisorToggle = screen.getByTestId("supervisor-enabled");
      await user.click(supervisorToggle);

      // Check sub-settings are disabled
      expect(screen.getByTestId("loop-threshold")).toBeDisabled();
      expect(screen.getByTestId("stuck-timeout")).toBeDisabled();
    });

    it("updates loop threshold", () => {
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      const input = screen.getByTestId("loop-threshold");
      fireEvent.change(input, { target: { value: "5" } });

      expect(onChange).toHaveBeenCalledTimes(1);
      const calledWith = onChange.mock.calls[0][0];
      expect(calledWith.supervisor.loop_threshold).toBe(5);
    });
  });

  describe("Initial Settings", () => {
    it("uses provided initial settings", () => {
      const customSettings = {
        ...DEFAULT_PROJECT_SETTINGS,
        execution: {
          ...DEFAULT_PROJECT_SETTINGS.execution,
          max_concurrent_tasks: 5,
          auto_commit: false,
        },
      };

      render(<SettingsView initialSettings={customSettings} />);

      expect(screen.getByTestId("max-concurrent-tasks")).toHaveValue(5);
      expect(screen.getByTestId("auto-commit")).toHaveAttribute("aria-checked", "false");
    });
  });

  describe("Disabled State", () => {
    it("disables all inputs when isSaving is true", () => {
      render(<SettingsView isSaving={true} />);

      // Check toggles are disabled
      expect(screen.getByTestId("auto-commit")).toHaveAttribute("aria-disabled", "true");
      expect(screen.getByTestId("ai-review-enabled")).toHaveAttribute("aria-disabled", "true");
      expect(screen.getByTestId("supervisor-enabled")).toHaveAttribute("aria-disabled", "true");

      // Check number inputs are disabled
      expect(screen.getByTestId("max-concurrent-tasks")).toBeDisabled();
      expect(screen.getByTestId("model-selection")).toBeDisabled();
    });
  });

  describe("Accessibility", () => {
    it("has proper aria attributes on toggles", () => {
      render(<SettingsView />);

      const toggle = screen.getByTestId("auto-commit");
      expect(toggle).toHaveAttribute("role", "switch");
      expect(toggle).toHaveAttribute("aria-checked");
    });

    it("associates descriptions with inputs", () => {
      render(<SettingsView />);

      const autoCommitToggle = screen.getByTestId("auto-commit");
      const descId = autoCommitToggle.getAttribute("aria-describedby");
      expect(descId).toBeTruthy();

      const description = document.getElementById(descId!);
      expect(description).toBeInTheDocument();
      expect(description?.textContent).toContain("commit");
    });

    it("handles keyboard navigation on toggles", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      const toggle = screen.getByTestId("auto-commit");
      toggle.focus();

      await user.keyboard(" ");
      expect(onChange).toHaveBeenCalled();
    });
  });
});
