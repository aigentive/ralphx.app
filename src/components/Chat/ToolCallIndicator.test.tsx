import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";

/**
 * ToolCallIndicator tests.
 *
 * Tools with dedicated widgets (bash, read, grep, glob, step tools, context,
 * artifacts, reviews, proposals, merges, ideation) are routed to their widgets
 * by the registry in tool-widgets/registry.ts. These tests cover the GENERIC
 * fallback renderer — used for tools without a widget (update_task, add_task_note,
 * custom_tool, etc.) and for edit/write error fallback cases.
 */
describe("ToolCallIndicator", () => {
  describe("Rendering (generic fallback)", () => {
    it("renders collapsed by default", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "update_task",
        arguments: { task_id: "task-1" },
        result: { ok: true },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      expect(screen.getByText("update_task")).toBeInTheDocument();
      expect(screen.getByText("Updated task")).toBeInTheDocument();

      // Should NOT show details initially
      expect(screen.queryByTestId("tool-call-details")).not.toBeInTheDocument();
    });

    it("shows tool icon and chevron", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { data: "test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");
      expect(toggle).toBeInTheDocument();
    });

    it("applies custom className", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { data: "test" },
      };

      const { container } = render(
        <ToolCallIndicator toolCall={toolCall} className="custom-class" />
      );

      const indicator = container.querySelector(".custom-class");
      expect(indicator).toBeInTheDocument();
    });
  });

  describe("Interaction (generic fallback)", () => {
    it("expands when clicked", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "update_task",
        arguments: { task_id: "task-1" },
        result: { ok: true },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");
      await user.click(toggle);

      // Details should now be visible
      expect(screen.getByTestId("tool-call-details")).toBeInTheDocument();

      // Should show arguments label
      expect(screen.getByText("Arguments")).toBeInTheDocument();

      // Should show result label
      expect(screen.getByText("Result")).toBeInTheDocument();
    });

    it("collapses when clicked again", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "add_task_note",
        arguments: { task_id: "task-abc", note: "test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");

      // Expand
      await user.click(toggle);
      expect(screen.getByTestId("tool-call-details")).toBeInTheDocument();

      // Collapse
      await user.click(toggle);
      expect(screen.queryByTestId("tool-call-details")).not.toBeInTheDocument();
    });

    it("has correct aria-expanded attribute", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { data: "test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");

      // Initially collapsed
      expect(toggle).toHaveAttribute("aria-expanded", "false");

      // After click, expanded
      await user.click(toggle);
      expect(toggle).toHaveAttribute("aria-expanded", "true");
    });
  });

  describe("Summary generation", () => {
    it("shows file path for write tool", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "write",
        arguments: { file_path: "/app/config.json" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("write")).toBeInTheDocument();
      expect(screen.getByText(/\/app\/config.json/i)).toBeInTheDocument();
    });

    it("shows file path for edit tool", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "edit",
        arguments: { file_path: "/src/main.rs" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("edit")).toBeInTheDocument();
      expect(screen.getByText(/\/src\/main.rs/i)).toBeInTheDocument();
    });

    it("shows update_task summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "update_task",
        arguments: { task_id: "task-789" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("update_task")).toBeInTheDocument();
      expect(screen.getByText(/Updated task/i)).toBeInTheDocument();
    });

    it("shows add_task_note summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "add_task_note",
        arguments: { task_id: "task-abc", note: "Fixed the bug" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("add_task_note")).toBeInTheDocument();
      expect(screen.getByText(/Added note/i)).toBeInTheDocument();
    });

    it("shows tool name for unknown tools", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { foo: "bar" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("custom_tool")).toBeInTheDocument();
      // Shows formatted tool name as summary (underscores replaced with spaces)
      expect(screen.getByText("custom tool")).toBeInTheDocument();
    });
  });

  describe("Expanded details", () => {
    it("displays tool name badge in collapsed view", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { action: "test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      // Tool name badge should be visible in collapsed view
      expect(screen.getByText("custom_tool")).toBeInTheDocument();
    });

    it("displays formatted arguments when expanded", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: {
          title: "Test Task",
          description: "A test description",
          priority: "high",
        },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      await user.click(screen.getByTestId("tool-call-toggle"));

      expect(screen.getByText("Arguments")).toBeInTheDocument();

      // JSON should be formatted
      const detailsContainer = screen.getByTestId("tool-call-details");
      expect(detailsContainer.textContent).toContain("Test Task");
      expect(detailsContainer.textContent).toContain("high");
    });

    it("displays result when present", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { action: "get" },
        result: "hello\n",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      await user.click(screen.getByTestId("tool-call-toggle"));

      expect(screen.getByText("Result")).toBeInTheDocument();
      const detailsContainer = screen.getByTestId("tool-call-details");
      expect(detailsContainer.textContent).toContain("hello");
    });

    it("does not display result section when result is undefined", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { action: "get" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      await user.click(screen.getByTestId("tool-call-toggle"));

      expect(screen.queryByText("Result")).not.toBeInTheDocument();
    });
  });

  describe("Error handling", () => {
    it("displays error indicator when tool call failed", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { action: "fail" },
        error: "Command not found",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      expect(screen.getByText("Failed")).toBeInTheDocument();
    });

    it("displays error message in expanded view", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { path: "/nonexistent.txt" },
        error: "File not found: /nonexistent.txt",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      await user.click(screen.getByTestId("tool-call-toggle"));

      expect(screen.getByText("Error")).toBeInTheDocument();
      const detailsContainer = screen.getByTestId("tool-call-details");
      expect(detailsContainer.textContent).toContain("File not found");
    });

    it("does not display result when error is present", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { action: "list" },
        result: "should not show",
        error: "Some error occurred",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      await user.click(screen.getByTestId("tool-call-toggle"));

      // Should show error, not result
      expect(screen.getByText("Error")).toBeInTheDocument();
      expect(screen.queryByText("Result")).not.toBeInTheDocument();
    });

    it("applies error styling when tool call failed", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { action: "fail" },
        error: "Command failed",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const indicator = screen.getByTestId("tool-call-indicator");
      // Error state uses a red-tinted background
      expect(indicator).toHaveStyle({ backgroundColor: "hsla(0 70% 55% / 0.15)" });
    });
  });

  describe("Accessibility", () => {
    it("has accessible label for toggle button", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { data: "test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");
      expect(toggle).toHaveAttribute("aria-label");
      expect(toggle.getAttribute("aria-label")).toContain("custom_tool");
      expect(toggle.getAttribute("aria-label")).toContain("expand");
    });

    it("updates aria-label when expanded", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        arguments: { data: "test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");

      // Initially should say "expand"
      expect(toggle.getAttribute("aria-label")).toContain("expand");

      // After click, should say "collapse"
      await user.click(toggle);
      expect(toggle.getAttribute("aria-label")).toContain("collapse");
    });
  });
});
