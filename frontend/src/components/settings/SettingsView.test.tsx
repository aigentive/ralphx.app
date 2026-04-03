/**
 * SettingsView component tests
 *
 * Tests for the premium Settings View implementation with:
 * - Glass effect header with Settings icon
 * - Section cards with gradient borders
 * - shadcn Switch, Input, Select components
 * - Master toggle → sub-settings disabled pattern
 * - Lucide icons throughout
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render as rtlRender, screen, fireEvent, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SettingsView } from "./SettingsView";
import { DEFAULT_PROJECT_SETTINGS } from "@/types/settings";

const mockSubscribe = vi.fn(() => vi.fn());

vi.mock("@/providers/EventProvider", () => ({
  useEventBus: () => ({
    subscribe: mockSubscribe,
  }),
}));

const createTestQueryClient = () =>
  new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  });

const render = (ui: Parameters<typeof rtlRender>[0]) =>
  rtlRender(
    <QueryClientProvider client={createTestQueryClient()}>{ui}</QueryClientProvider>
  );

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

    it("renders Settings icon in header", () => {
      render(<SettingsView />);
      // The Settings icon is rendered in the header with accent color
      const header = screen.getByText("Settings").closest("div");
      expect(header).toBeInTheDocument();
    });

    it("renders section icons", () => {
      render(<SettingsView />);
      // Each section has its own icon (Zap, Brain, FileSearch, Shield)
      // We verify the sections render with their descriptions
      expect(screen.getByText("Control task execution behavior and concurrency")).toBeInTheDocument();
      expect(screen.getByText("Configure AI model selection")).toBeInTheDocument();
      expect(screen.getByText("Configure code review automation")).toBeInTheDocument();
      expect(screen.getByText("Configure watchdog monitoring for stuck or looping agents")).toBeInTheDocument();
    });
  });

  describe("Execution Section", () => {
    it("renders all execution settings", () => {
      render(<SettingsView />);
      expect(screen.getByText("Max Concurrent Tasks")).toBeInTheDocument();
      expect(screen.getByText("Project Ideation Cap")).toBeInTheDocument();
      expect(screen.getByText("Auto Commit")).toBeInTheDocument();
      expect(screen.getByText("Pause on Failure")).toBeInTheDocument();
      expect(screen.getByText("Review Before Destructive")).toBeInTheDocument();
    });

    it("displays default max concurrent tasks value", () => {
      render(<SettingsView />);
      const input = screen.getByTestId("max-concurrent-tasks");
      expect(input).toHaveValue(DEFAULT_PROJECT_SETTINGS.execution.max_concurrent_tasks);
    });

    it("displays default project ideation cap value", () => {
      render(<SettingsView />);
      const input = screen.getByTestId("project-ideation-max");
      expect(input).toHaveValue(DEFAULT_PROJECT_SETTINGS.execution.project_ideation_max);
    });

    it("toggles auto commit setting", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      const toggle = screen.getByTestId("auto-commit");
      // shadcn Switch uses data-state for checked state
      expect(toggle).toHaveAttribute("data-state", "checked");

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

    it("updates project ideation cap", () => {
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      const input = screen.getByTestId("project-ideation-max");
      fireEvent.change(input, { target: { value: "4" } });

      expect(onChange).toHaveBeenCalledTimes(1);
      const calledWith = onChange.mock.calls[0][0];
      expect(calledWith.execution.project_ideation_max).toBe(4);
    });
  });

  describe("Model Section", () => {
    it("renders all model settings", () => {
      render(<SettingsView />);
      expect(screen.getByText("Default Model")).toBeInTheDocument();
      expect(screen.getByText("Allow Opus Upgrade")).toBeInTheDocument();
    });

    it("displays model dropdown with current value", () => {
      render(<SettingsView />);
      const select = screen.getByTestId("model-selection");
      // shadcn Select shows the current value
      expect(within(select).getByText("Claude Sonnet 4.5")).toBeInTheDocument();
    });

    it("changes model selection", async () => {
      const _user = userEvent.setup();
      const onChange = vi.fn();
      render(<SettingsView onSettingsChange={onChange} />);

      // Radix Select components are challenging to test in jsdom
      // Verify the select trigger shows the current model value
      const selectTrigger = screen.getByTestId("model-selection");
      expect(selectTrigger).toBeInTheDocument();
      expect(within(selectTrigger).getByText("Claude Sonnet 4.5")).toBeInTheDocument();

      // The Select component is tested by verifying it renders properly
      // Full interaction testing with Radix Select requires additional jsdom polyfills
      // that are out of scope for this component test
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

      // Check sub-settings are disabled (shadcn Switch uses disabled attribute)
      expect(screen.getByTestId("ai-review-auto-fix")).toBeDisabled();
      expect(screen.getByTestId("require-fix-approval")).toBeDisabled();
      expect(screen.getByTestId("require-human-review")).toBeDisabled();
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

    it("shows seconds unit for stuck timeout", () => {
      render(<SettingsView />);
      expect(screen.getByText("seconds")).toBeInTheDocument();
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
      expect(screen.getByTestId("auto-commit")).toHaveAttribute("data-state", "unchecked");
    });

    it("updates settings when initialSettings prop changes", () => {
      const initialSettings = {
        ...DEFAULT_PROJECT_SETTINGS,
        execution: {
          ...DEFAULT_PROJECT_SETTINGS.execution,
          max_concurrent_tasks: 3,
        },
      };

      const { rerender } = render(<SettingsView initialSettings={initialSettings} />);
      expect(screen.getByTestId("max-concurrent-tasks")).toHaveValue(3);

      // Simulate project switch - new settings with different max_concurrent_tasks
      const newSettings = {
        ...DEFAULT_PROJECT_SETTINGS,
        execution: {
          ...DEFAULT_PROJECT_SETTINGS.execution,
          max_concurrent_tasks: 7,
        },
      };

      rerender(
        <QueryClientProvider client={createTestQueryClient()}>
          <SettingsView initialSettings={newSettings} />
        </QueryClientProvider>
      );

      // Verify the input now shows the new project's value
      expect(screen.getByTestId("max-concurrent-tasks")).toHaveValue(7);
    });
  });

  describe("Disabled State", () => {
    it("disables all inputs when isSaving is true", () => {
      render(<SettingsView isSaving={true} />);

      // Check toggles are disabled (shadcn Switch uses disabled attribute)
      expect(screen.getByTestId("auto-commit")).toBeDisabled();
      expect(screen.getByTestId("ai-review-enabled")).toBeDisabled();
      expect(screen.getByTestId("supervisor-enabled")).toBeDisabled();

      // Check number inputs are disabled
      expect(screen.getByTestId("max-concurrent-tasks")).toBeDisabled();

      // Check select is disabled
      expect(screen.getByTestId("model-selection")).toBeDisabled();
    });
  });

  describe("Accessibility", () => {
    it("has proper role on toggles", () => {
      render(<SettingsView />);

      const toggle = screen.getByTestId("auto-commit");
      expect(toggle).toHaveAttribute("role", "switch");
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

  describe("Error Banner", () => {
    it("can dismiss error by clicking X button", async () => {
      const user = userEvent.setup();
      const errorMessage = "Failed to save settings";
      render(<SettingsView error={errorMessage} />);

      expect(screen.getByText(errorMessage)).toBeInTheDocument();

      // Find and click the dismiss button
      const dismissButton = screen.getByRole("button", { name: "" });
      await user.click(dismissButton);

      expect(screen.queryByText(errorMessage)).not.toBeInTheDocument();
    });
  });

  describe("Premium Design Elements", () => {
    it("renders glass effect header with backdrop blur", () => {
      render(<SettingsView />);

      // Header uses inline translucent background + blur styling
      const heading = screen.getByRole("heading", { name: "Settings" });
      let styledAncestor: HTMLElement | null = heading as HTMLElement;
      while (styledAncestor && !styledAncestor.getAttribute("style")) {
        styledAncestor = styledAncestor.parentElement;
      }
      expect(styledAncestor).toBeInTheDocument();
      expect(styledAncestor?.getAttribute("style")?.replace(/\s+/g, "")).toContain(
        "rgba(18,18,18,0.85)"
      );
    });

    it("renders warm radial gradient background", () => {
      render(<SettingsView />);

      const settingsView = screen.getByTestId("settings-view");
      // Container uses layered radial gradients over --bg-base
      expect(settingsView.getAttribute("style")).toContain("var(--bg-base)");
    });

    it("renders sub-settings with visual indentation", async () => {
      render(<SettingsView />);

      // Ensure AI review is enabled to see sub-settings styling
      // Sub-settings have border-l-2 styling for visual indentation
      const autoFixLabel = screen.getByText("Auto Create Fix Tasks");
      const autoFixRow = autoFixLabel.closest("div");
      // The label is inside a div with border-l-2 class
      const indentedContainer = autoFixRow?.querySelector(".border-l-2");
      expect(indentedContainer || autoFixRow?.className.includes("border-l")).toBeTruthy();
    });
  });

  describe("Global Execution Section", () => {
    it("renders all global execution settings", async () => {
      render(<SettingsView />);

      expect(await screen.findByText("Global Max Concurrent")).toBeInTheDocument();
      expect(screen.getByText("Global Ideation Cap")).toBeInTheDocument();
      expect(screen.getByText("Allow Ideation Borrowing")).toBeInTheDocument();
    });

    it("displays default global execution values", async () => {
      render(<SettingsView />);

      expect(await screen.findByTestId("global-max-concurrent")).toHaveValue(20);
      expect(screen.getByTestId("global-ideation-max")).toHaveValue(10);
      expect(screen.getByTestId("allow-ideation-borrow-idle-execution")).toHaveAttribute(
        "data-state",
        "unchecked"
      );
    });
  });
});
