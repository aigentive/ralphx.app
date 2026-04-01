/**
 * TeammateCard tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TeammateCard } from "./TeammateCard";
import type { TeammateState } from "@/stores/teamStore";

function makeTeammate(overrides: Partial<TeammateState> = {}): TeammateState {
  return {
    name: "coder-1",
    color: "#3b82f6",
    model: "sonnet",
    roleDescription: "Auth middleware",
    status: "running",
    currentActivity: "Writing auth.ts",
    tokensUsed: 50000,
    estimatedCostUsd: 0.3,
    conversationId: null,
    ...overrides,
  };
}

describe("TeammateCard", () => {
  it("renders name, model, and status", () => {
    render(<TeammateCard teammate={makeTeammate()} />);
    expect(screen.getByText("coder-1")).toBeInTheDocument();
    expect(screen.getByText("sonnet")).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
  });

  it("renders role description", () => {
    render(<TeammateCard teammate={makeTeammate({ roleDescription: "Auth middleware" })} />);
    expect(screen.getByText("Auth middleware")).toBeInTheDocument();
  });

  it("renders current activity when running", () => {
    render(<TeammateCard teammate={makeTeammate({ currentActivity: "Writing auth.ts" })} />);
    expect(screen.getByText("Writing auth.ts")).toBeInTheDocument();
  });

  it("hides current activity when shutdown", () => {
    render(
      <TeammateCard
        teammate={makeTeammate({ status: "shutdown", currentActivity: "Writing auth.ts" })}
      />,
    );
    expect(screen.queryByText("Writing auth.ts")).not.toBeInTheDocument();
  });

  it("renders cost and tokens", () => {
    render(<TeammateCard teammate={makeTeammate({ tokensUsed: 50000, estimatedCostUsd: 0.3 })} />);
    expect(screen.getByText(/~50K tokens/)).toBeInTheDocument();
    expect(screen.getByText(/\$0\.30/)).toBeInTheDocument();
  });

  it("renders <$0.01 for very small costs", () => {
    render(<TeammateCard teammate={makeTeammate({ tokensUsed: 100, estimatedCostUsd: 0.001 })} />);
    expect(screen.getByText(/<\$0\.01/)).toBeInTheDocument();
  });

  it("renders message button when onMessage provided", () => {
    const onMessage = vi.fn();
    render(<TeammateCard teammate={makeTeammate()} onMessage={onMessage} />);
    const btn = screen.getByLabelText("Message coder-1");
    expect(btn).toBeInTheDocument();
    fireEvent.click(btn);
    expect(onMessage).toHaveBeenCalledWith("coder-1");
  });

  it("renders stop button when running and onStop provided", () => {
    const onStop = vi.fn();
    render(<TeammateCard teammate={makeTeammate({ status: "running" })} onStop={onStop} />);
    const btn = screen.getByLabelText("Stop coder-1");
    expect(btn).toBeInTheDocument();
    fireEvent.click(btn);
    expect(onStop).toHaveBeenCalledWith("coder-1");
  });

  it("does not render stop button for completed teammate", () => {
    const onStop = vi.fn();
    render(<TeammateCard teammate={makeTeammate({ status: "completed" })} onStop={onStop} />);
    expect(screen.queryByLabelText("Stop coder-1")).not.toBeInTheDocument();
  });

  it("does not render action buttons for shutdown teammate", () => {
    const onMessage = vi.fn();
    const onStop = vi.fn();
    render(
      <TeammateCard
        teammate={makeTeammate({ status: "shutdown" })}
        onMessage={onMessage}
        onStop={onStop}
      />,
    );
    expect(screen.queryByLabelText("Message coder-1")).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Stop coder-1")).not.toBeInTheDocument();
  });

  it("renders all status labels correctly", () => {
    const statuses: Array<[TeammateState["status"], string]> = [
      ["spawning", "Spawning"],
      ["running", "Running"],
      ["idle", "Idle"],
      ["completed", "Done"],
      ["failed", "Failed"],
      ["shutdown", "Stopped"],
    ];
    for (const [status, label] of statuses) {
      const { unmount } = render(<TeammateCard teammate={makeTeammate({ status })} />);
      expect(screen.getByText(label)).toBeInTheDocument();
      unmount();
    }
  });
});
