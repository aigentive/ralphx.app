/**
 * TargetSelector tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TargetSelector } from "./TargetSelector";
import type { TeammateState } from "@/stores/teamStore";

function makeTeammate(overrides: Partial<TeammateState> = {}): TeammateState {
  return {
    name: "coder-1",
    color: "#3b82f6",
    model: "sonnet",
    roleDescription: "Auth",
    status: "running",
    currentActivity: null,
    tokensUsed: 0,
    estimatedCostUsd: 0,
    streamingText: "",
    ...overrides,
  };
}

describe("TargetSelector", () => {
  it("renders Send to: label with current value", () => {
    render(
      <TargetSelector teammates={[]} value="lead" onChange={vi.fn()} />,
    );
    expect(screen.getByText("Send to:")).toBeInTheDocument();
    expect(screen.getByText("Lead")).toBeInTheDocument();
  });

  it("displays 'All' for broadcast value", () => {
    render(
      <TargetSelector teammates={[]} value="*" onChange={vi.fn()} />,
    );
    expect(screen.getByText("All")).toBeInTheDocument();
  });

  it("displays teammate name for custom value", () => {
    const teammates = [makeTeammate({ name: "coder-1" })];
    render(
      <TargetSelector teammates={teammates} value="coder-1" onChange={vi.fn()} />,
    );
    expect(screen.getByText("coder-1")).toBeInTheDocument();
  });

  it("opens dropdown on click", () => {
    const teammates = [makeTeammate({ name: "coder-1" })];
    render(
      <TargetSelector teammates={teammates} value="lead" onChange={vi.fn()} />,
    );
    // Dropdown not yet visible
    expect(screen.queryByText("All (broadcast)")).not.toBeInTheDocument();
    // Click to open
    fireEvent.click(screen.getByText("Lead"));
    // Now dropdown is visible
    expect(screen.getByText("All (broadcast)")).toBeInTheDocument();
  });

  it("calls onChange when selecting a teammate", () => {
    const onChange = vi.fn();
    const teammates = [makeTeammate({ name: "coder-1" })];
    render(
      <TargetSelector teammates={teammates} value="lead" onChange={onChange} />,
    );
    fireEvent.click(screen.getByText("Lead"));
    // Click coder-1 in dropdown
    fireEvent.click(screen.getByText("coder-1"));
    expect(onChange).toHaveBeenCalledWith("coder-1");
  });

  it("calls onChange with * when selecting All (broadcast)", () => {
    const onChange = vi.fn();
    render(
      <TargetSelector teammates={[]} value="lead" onChange={onChange} />,
    );
    fireEvent.click(screen.getByText("Lead"));
    fireEvent.click(screen.getByText("All (broadcast)"));
    expect(onChange).toHaveBeenCalledWith("*");
  });

  it("closes dropdown after selection", () => {
    const onChange = vi.fn();
    render(
      <TargetSelector teammates={[]} value="lead" onChange={onChange} />,
    );
    fireEvent.click(screen.getByText("Lead"));
    expect(screen.getByText("All (broadcast)")).toBeInTheDocument();
    fireEvent.click(screen.getByText("All (broadcast)"));
    expect(screen.queryByText("All (broadcast)")).not.toBeInTheDocument();
  });

  it("filters out shutdown teammates from dropdown", () => {
    const teammates = [
      makeTeammate({ name: "coder-1", status: "running" }),
      makeTeammate({ name: "coder-2", status: "shutdown", color: "#10b981" }),
    ];
    render(
      <TargetSelector teammates={teammates} value="lead" onChange={vi.fn()} />,
    );
    fireEvent.click(screen.getByText("Lead"));
    expect(screen.getByText("coder-1")).toBeInTheDocument();
    // coder-2 is shutdown, should not be in dropdown
    // Note: "Lead" already shown above, so checking specifically in dropdown context
    const buttons = screen.getAllByRole("button");
    const labels = buttons.map((b) => b.textContent);
    expect(labels).not.toContain("coder-2");
  });
});
