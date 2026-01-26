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
        input: { command: "ls -la" },
        result: "file1.txt\nfile2.txt",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      // Should show summary
      expect(screen.getByText(/Ran command: ls -la/i)).toBeInTheDocument();

      // Should NOT show details initially
      expect(screen.queryByTestId("tool-call-details")).not.toBeInTheDocument();
    });

    it("shows wrench icon and chevron", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "read",
        input: { file_path: "/path/to/file.txt" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");
      expect(toggle).toBeInTheDocument();
    });

    it("applies custom className", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        input: { command: "echo hello" },
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
        input: { command: "pwd" },
        result: "/home/user",
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      const toggle = screen.getByTestId("tool-call-toggle");
      await user.click(toggle);

      // Details should now be visible
      expect(screen.getByTestId("tool-call-details")).toBeInTheDocument();

      // Should show tool name
      expect(screen.getByText("bash")).toBeInTheDocument();

      // Should show arguments
      expect(screen.getByText("Arguments")).toBeInTheDocument();

      // Should show result
      expect(screen.getByText("Result")).toBeInTheDocument();
    });

    it("collapses when clicked again", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "read",
        input: { file_path: "/test.txt" },
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
        input: { command: "ls" },
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
    it("formats bash command summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        input: { command: "npm install" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Ran command: npm install/i)).toBeInTheDocument();
    });

    it("formats read file summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "read",
        input: { file_path: "/Users/test/file.ts" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Read file: \/Users\/test\/file.ts/i)).toBeInTheDocument();
    });

    it("formats write file summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "write",
        input: { file_path: "/app/config.json" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Wrote file: \/app\/config.json/i)).toBeInTheDocument();
    });

    it("formats edit file summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "edit",
        input: { file_path: "/src/main.rs" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Edited file: \/src\/main.rs/i)).toBeInTheDocument();
    });

    it("formats create_task_proposal summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "create_task_proposal",
        input: { title: "Add dark mode" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Created proposal: Add dark mode/i)).toBeInTheDocument();
    });

    it("formats update_task_proposal summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "update_task_proposal",
        input: { proposal_id: "prop-123" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Updated proposal: prop-123/i)).toBeInTheDocument();
    });

    it("formats delete_task_proposal summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "delete_task_proposal",
        input: { proposal_id: "prop-456" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Deleted proposal: prop-456/i)).toBeInTheDocument();
    });

    it("formats update_task summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "update_task",
        input: { task_id: "task-789" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Updated task: task-789/i)).toBeInTheDocument();
    });

    it("formats add_task_note summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "add_task_note",
        input: { task_id: "task-abc", note: "Fixed the bug" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Added note to task: task-abc/i)).toBeInTheDocument();
    });

    it("formats unknown tool summary", () => {
      const toolCall: ToolCall = {
        id: "call-1",
        name: "custom_tool",
        input: { foo: "bar" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      expect(screen.getByText(/Called custom_tool/i)).toBeInTheDocument();
    });

    it("truncates long command summaries", () => {
      const longCommand = "a".repeat(100);
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        input: { command: longCommand },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);
      const summary = screen.getByText(/Ran command:/i);
      expect(summary.textContent).toContain("...");
    });
  });

  describe("Expanded details", () => {
    it("displays tool name in expanded view", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "bash",
        input: { command: "echo test" },
      };

      render(<ToolCallIndicator toolCall={toolCall} />);

      await user.click(screen.getByTestId("tool-call-toggle"));

      expect(screen.getByText("Tool")).toBeInTheDocument();
      expect(screen.getByText("bash")).toBeInTheDocument();
    });

    it("displays formatted arguments", async () => {
      const user = userEvent.setup();
      const toolCall: ToolCall = {
        id: "call-1",
        name: "create_task_proposal",
        input: {
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
        input: { command: "echo hello" },
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
        input: { command: "ls" },
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
        input: { command: "invalid-command" },
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
        input: { file_path: "/nonexistent.txt" },
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
        input: { command: "ls" },
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
        input: { command: "fail" },
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
        input: { command: "ls" },
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
        input: { file_path: "/test.txt" },
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
