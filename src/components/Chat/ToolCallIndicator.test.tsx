import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";

describe("ToolCallIndicator", () => {
  describe("Rendering", () => {
    it("renders collapsed by default", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: "ls -la" },
        result: "file1.txt\nfile2.txt",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      // Should show tool name badge and summary (command text)
      expect(screen.getByText("bash")).toBeInTheDocument();
      expect(screen.getByText(/ls -la/i)).toBeInTheDocument();

      // Should NOT show details initially
      expect(screen.queryByTestId("tool-call-details")).not.toBeInTheDocument();
    });

    it("shows tool icon and chevron", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "read",
        arguments: { file_path: "/path/to/file.txt" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");
      expect(toggle).toBeInTheDocument();
    });

    it("applies custom className", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: "echo hello" },
      };

      const { container } = render(
        <ToolCallIndicator toolCall={toolCall} className="custom-class" />
      );

      const indicator = container.querySelector(".custom-class");
      expect(indicator).toBeInTheDocument();
    });
  });

  describe("Interaction", () => {
    it("expands when clicked", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: "pwd" },
        result: "/home/user",
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
        name: "read",
        arguments: { file_path: "/test.txt" },
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
        name: "bash",
        arguments: { command: "ls" },
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
    it("shows bash command in summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: "npm install" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("bash")).toBeInTheDocument();
      expect(screen.getByText(/npm install/i)).toBeInTheDocument();
    });

    it("shows bash description when provided", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: "npm install", description: "Install dependencies" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Install dependencies/i)).toBeInTheDocument();
    });

    it("shows file path for read tool", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "read",
        arguments: { file_path: "/Users/test/file.ts" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("read")).toBeInTheDocument();
      expect(screen.getByText(/\/Users\/test\/file.ts/i)).toBeInTheDocument();
    });

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

    it("shows title for create_task_proposal", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "create_task_proposal",
        arguments: { title: "Add dark mode" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("create_task_proposal")).toBeInTheDocument();
      expect(screen.getByText(/Add dark mode/i)).toBeInTheDocument();
    });

    it("shows title for update_task_proposal", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "update_task_proposal",
        arguments: { title: "Updated proposal title" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("update_task_proposal")).toBeInTheDocument();
      expect(screen.getByText(/Updated proposal title/i)).toBeInTheDocument();
    });

    it("shows delete_task_proposal summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "delete_task_proposal",
        arguments: { proposal_id: "prop-456" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText("delete_task_proposal")).toBeInTheDocument();
      expect(screen.getByText(/Deleted proposal/i)).toBeInTheDocument();
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
      // Shows extracted argument value
      expect(screen.getByText(/bar/i)).toBeInTheDocument();
    });

    it("truncates long command summaries", () => {
      const longCommand = "a".repeat(100);
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: longCommand },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      // Should show truncated version with ellipsis
      const indicator = screen.getByTestId("tool-call-indicator");
      expect(indicator.textContent).toContain("...");
    });
  });

  describe("Expanded details", () => {
    it("displays tool name badge in collapsed view", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: "echo test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      // Tool name badge should be visible in collapsed view
      expect(screen.getByText("bash")).toBeInTheDocument();
    });

    it("displays formatted arguments when expanded", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "create_task_proposal",
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
        name: "bash",
        arguments: { command: "echo hello" },
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
        name: "bash",
        arguments: { command: "ls" },
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
        name: "bash",
        arguments: { command: "invalid-command" },
        error: "Command not found",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      expect(screen.getByText("Failed")).toBeInTheDocument();
    });

    it("displays error message in expanded view", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "read",
        arguments: { file_path: "/nonexistent.txt" },
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
        name: "bash",
        arguments: { command: "ls" },
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
        name: "bash",
        arguments: { command: "fail" },
        error: "Command failed",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const indicator = screen.getByTestId("tool-call-indicator");
      expect(indicator).toHaveStyle({ opacity: "0.9" });
    });
  });

  describe("Accessibility", () => {
    it("has accessible label for toggle button", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        arguments: { command: "ls" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");
      expect(toggle).toHaveAttribute("aria-label");
      expect(toggle.getAttribute("aria-label")).toContain("bash");
      expect(toggle.getAttribute("aria-label")).toContain("expand");
    });

    it("updates aria-label when expanded", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "read",
        arguments: { file_path: "/test.txt" },
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
