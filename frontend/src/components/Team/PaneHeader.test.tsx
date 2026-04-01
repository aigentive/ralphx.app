import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PaneHeader } from "./PaneHeader";
import type { TeammateStatus } from "@/stores/teamStore";

const defaultProps = {
  name: "worker-1",
  color: "#4ade80",
  model: "sonnet-4",
  status: "running" as TeammateStatus,
  roleDescription: "Frontend coder",
};

describe("PaneHeader", () => {
  it("renders name, model badge, and role description", () => {
    render(<PaneHeader {...defaultProps} />);
    expect(screen.getByText("worker-1")).toBeInTheDocument();
    expect(screen.getByText("sonnet-4")).toBeInTheDocument();
    expect(screen.getByText("Frontend coder")).toBeInTheDocument();
  });

  it("renders color dot with correct background color", () => {
    const { container } = render(<PaneHeader {...defaultProps} />);
    const dot = container.querySelector(".rounded-full") as HTMLElement;
    expect(dot.style.backgroundColor).toBe("rgb(74, 222, 128)");
  });

  it.each([
    ["spawning", "spawning"],
    ["running", "running"],
    ["idle", "idle"],
    ["completed", "done"],
    ["failed", "failed"],
    ["shutdown", "stopped"],
  ] as [TeammateStatus, string][])("renders STATUS_DISPLAY label for %s as %s", (status, label) => {
    render(<PaneHeader {...defaultProps} status={status} />);
    expect(screen.getByText(label)).toBeInTheDocument();
  });

  it("shows pulse animation only for running status", () => {
    const { container, rerender } = render(<PaneHeader {...defaultProps} status="running" />);
    const statusDot = container.querySelectorAll(".rounded-full")[1] as HTMLElement;
    expect(statusDot.className).toContain("animate-pulse");

    rerender(<PaneHeader {...defaultProps} status="idle" />);
    const idleDot = container.querySelectorAll(".rounded-full")[1] as HTMLElement;
    expect(idleDot.className).not.toContain("animate-pulse");
  });

  it("shows stop button on hover for running status", () => {
    const onStop = vi.fn();
    render(<PaneHeader {...defaultProps} status="running" onStop={onStop} />);
    const stopBtn = screen.getByLabelText("Stop worker-1");
    expect(stopBtn).toBeInTheDocument();
    fireEvent.click(stopBtn);
    expect(onStop).toHaveBeenCalledTimes(1);
  });

  it("shows stop button for idle status", () => {
    const onStop = vi.fn();
    render(<PaneHeader {...defaultProps} status="idle" onStop={onStop} />);
    expect(screen.getByLabelText("Stop worker-1")).toBeInTheDocument();
  });

  it("hides stop button for shutdown/completed/failed/spawning statuses", () => {
    for (const status of ["shutdown", "completed", "failed", "spawning"] as TeammateStatus[]) {
      const { unmount } = render(<PaneHeader {...defaultProps} status={status} onStop={() => {}} />);
      expect(screen.queryByLabelText("Stop worker-1")).not.toBeInTheDocument();
      unmount();
    }
  });

  it("hides stop button when onStop is not provided", () => {
    render(<PaneHeader {...defaultProps} status="running" />);
    expect(screen.queryByLabelText("Stop worker-1")).not.toBeInTheDocument();
  });
});
