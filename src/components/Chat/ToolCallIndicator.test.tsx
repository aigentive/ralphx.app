import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ToolCallIndicator, type ToolCall } from "./ToolCallIndicator";

describe("ToolCallIndicator", () => {
  it("starts expanded for bash tool calls", () => {
    const toolCall: ToolCall = {
      id: "call-1",
      name: "bash",
      arguments: { command: "ls -la" },
      result: "file1.txt\nfile2.txt",
    };

    render(<ToolCallIndicator toolCall={toolCall} />);

    const toggle = screen.getByTestId("tool-call-toggle");
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId("tool-call-details")).toBeInTheDocument();
    expect(screen.getByText("bash")).toBeInTheDocument();
  });

  it("starts collapsed for non-bash tool calls", () => {
    const toolCall: ToolCall = {
      id: "call-2",
      name: "read",
      arguments: { file_path: "/tmp/file.txt" },
    };

    render(<ToolCallIndicator toolCall={toolCall} />);

    const toggle = screen.getByTestId("tool-call-toggle");
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId("tool-call-details")).not.toBeInTheDocument();
  });

  it("toggles details visibility", async () => {
    const user = userEvent.setup();
    const toolCall: ToolCall = {
      id: "call-3",
      name: "read",
      arguments: { file_path: "/tmp/file.txt" },
      result: "content",
    };

    render(<ToolCallIndicator toolCall={toolCall} />);

    const toggle = screen.getByTestId("tool-call-toggle");
    await user.click(toggle);
    expect(screen.getByTestId("tool-call-details")).toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-expanded", "true");

    await user.click(toggle);
    expect(screen.queryByTestId("tool-call-details")).not.toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-expanded", "false");
  });

  it("shows arguments and result in expanded view", async () => {
    const user = userEvent.setup();
    const toolCall: ToolCall = {
      id: "call-4",
      name: "read",
      arguments: { file_path: "/tmp/file.txt" },
      result: "hello",
    };

    render(<ToolCallIndicator toolCall={toolCall} />);
    await user.click(screen.getByTestId("tool-call-toggle"));

    expect(screen.getByText("Arguments")).toBeInTheDocument();
    expect(screen.getByText("Result")).toBeInTheDocument();
    expect(screen.getByTestId("tool-call-details").textContent).toContain("/tmp/file.txt");
    expect(screen.getByTestId("tool-call-details").textContent).toContain("hello");
  });

  it("shows error section and hides result when error is present", async () => {
    const user = userEvent.setup();
    const toolCall: ToolCall = {
      id: "call-5",
      name: "read",
      arguments: { file_path: "/missing.txt" },
      result: "should-not-show",
      error: "File not found",
    };

    render(<ToolCallIndicator toolCall={toolCall} />);
    await user.click(screen.getByTestId("tool-call-toggle"));

    expect(screen.getByText("Failed")).toBeInTheDocument();
    expect(screen.getByText("Error")).toBeInTheDocument();
    expect(screen.queryByText("Result")).not.toBeInTheDocument();
    expect(screen.getByTestId("tool-call-details").textContent).toContain("File not found");
  });

  it("updates aria-label between expand/collapse", async () => {
    const user = userEvent.setup();
    const toolCall: ToolCall = {
      id: "call-6",
      name: "read",
      arguments: { file_path: "/tmp/test.txt" },
    };

    render(<ToolCallIndicator toolCall={toolCall} />);

    const toggle = screen.getByTestId("tool-call-toggle");
    expect(toggle.getAttribute("aria-label")).toContain("expand");

    await user.click(toggle);
    expect(toggle.getAttribute("aria-label")).toContain("collapse");
  });

  it("applies custom className", () => {
    const toolCall: ToolCall = {
      id: "call-7",
      name: "read",
      arguments: { file_path: "/tmp/file.txt" },
    };

    const { container } = render(
      <ToolCallIndicator toolCall={toolCall} className="custom-class" />
    );

    expect(container.querySelector(".custom-class")).toBeInTheDocument();
  });
});
