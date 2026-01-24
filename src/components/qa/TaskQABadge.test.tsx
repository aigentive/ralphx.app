import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TaskQABadge, deriveQADisplayStatus } from "./TaskQABadge";

describe("deriveQADisplayStatus", () => {
  describe("with test status", () => {
    it("returns passed when test status is passed", () => {
      expect(deriveQADisplayStatus("completed", "passed")).toBe("passed");
    });

    it("returns failed when test status is failed", () => {
      expect(deriveQADisplayStatus("completed", "failed")).toBe("failed");
    });

    it("returns testing when test status is running", () => {
      expect(deriveQADisplayStatus("completed", "running")).toBe("testing");
    });

    it("falls through to prep status when test status is pending", () => {
      expect(deriveQADisplayStatus("running", "pending")).toBe("preparing");
      expect(deriveQADisplayStatus("completed", "pending")).toBe("ready");
    });
  });

  describe("with prep status only", () => {
    it("returns preparing when prep status is running", () => {
      expect(deriveQADisplayStatus("running")).toBe("preparing");
    });

    it("returns ready when prep status is completed", () => {
      expect(deriveQADisplayStatus("completed")).toBe("ready");
    });

    it("returns failed when prep status is failed", () => {
      expect(deriveQADisplayStatus("failed")).toBe("failed");
    });

    it("returns pending when prep status is pending", () => {
      expect(deriveQADisplayStatus("pending")).toBe("pending");
    });
  });

  describe("with no status", () => {
    it("returns pending when no status provided", () => {
      expect(deriveQADisplayStatus()).toBe("pending");
    });

    it("returns pending when undefined statuses", () => {
      expect(deriveQADisplayStatus(undefined, undefined)).toBe("pending");
    });
  });
});

describe("TaskQABadge", () => {
  it("renders nothing when needsQA is false", () => {
    render(<TaskQABadge needsQA={false} />);
    expect(screen.queryByTestId("task-qa-badge")).not.toBeInTheDocument();
  });

  it("renders badge when needsQA is true", () => {
    render(<TaskQABadge needsQA={true} />);
    expect(screen.getByTestId("task-qa-badge")).toBeInTheDocument();
  });

  it("shows pending status by default", () => {
    render(<TaskQABadge needsQA={true} />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "pending");
    expect(badge).toHaveTextContent("QA Pending");
  });

  it("shows preparing status when prep is running", () => {
    render(<TaskQABadge needsQA={true} prepStatus="running" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "preparing");
    expect(badge).toHaveTextContent("Preparing");
  });

  it("shows ready status when prep is completed", () => {
    render(<TaskQABadge needsQA={true} prepStatus="completed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "ready");
    expect(badge).toHaveTextContent("QA Ready");
  });

  it("shows testing status when test is running", () => {
    render(<TaskQABadge needsQA={true} testStatus="running" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "testing");
    expect(badge).toHaveTextContent("Testing");
  });

  it("shows passed status when test is passed", () => {
    render(<TaskQABadge needsQA={true} testStatus="passed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "passed");
    expect(badge).toHaveTextContent("Passed");
  });

  it("shows failed status when test is failed", () => {
    render(<TaskQABadge needsQA={true} testStatus="failed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "failed");
    expect(badge).toHaveTextContent("Failed");
  });

  it("shows failed status when prep is failed", () => {
    render(<TaskQABadge needsQA={true} prepStatus="failed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "failed");
    expect(badge).toHaveTextContent("Failed");
  });

  it("prioritizes test status over prep status", () => {
    render(<TaskQABadge needsQA={true} prepStatus="running" testStatus="passed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveAttribute("data-status", "passed");
    expect(badge).toHaveTextContent("Passed");
  });

  it("applies custom className", () => {
    render(<TaskQABadge needsQA={true} className="custom-class" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveClass("custom-class");
  });

  it("has correct base styling classes", () => {
    render(<TaskQABadge needsQA={true} />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveClass("inline-flex");
    expect(badge).toHaveClass("items-center");
    expect(badge).toHaveClass("px-2");
    expect(badge).toHaveClass("py-0.5");
    expect(badge).toHaveClass("rounded");
    expect(badge).toHaveClass("text-xs");
    expect(badge).toHaveClass("font-medium");
  });

  it("applies correct color class for passed status", () => {
    render(<TaskQABadge needsQA={true} testStatus="passed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveClass("bg-[--status-success]");
    expect(badge).toHaveClass("text-[--bg-base]");
  });

  it("applies correct color class for failed status", () => {
    render(<TaskQABadge needsQA={true} testStatus="failed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveClass("bg-[--status-error]");
    expect(badge).toHaveClass("text-[--bg-base]");
  });

  it("applies correct color class for preparing status", () => {
    render(<TaskQABadge needsQA={true} prepStatus="running" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveClass("bg-[--status-warning]");
  });

  it("applies correct color class for testing status", () => {
    render(<TaskQABadge needsQA={true} testStatus="running" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveClass("bg-[--accent-secondary]");
  });

  it("applies correct color class for ready status", () => {
    render(<TaskQABadge needsQA={true} prepStatus="completed" />);
    const badge = screen.getByTestId("task-qa-badge");
    expect(badge).toHaveClass("bg-[--status-info]");
  });
});
