/**
 * TeamFilterTabs tests
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { TeamFilterTabs } from "./TeamFilterTabs";
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
    conversationId: null,
    ...overrides,
  };
}

describe("TeamFilterTabs", () => {
  it("renders All and Lead tabs", () => {
    render(
      <TeamFilterTabs teammates={[]} activeFilter="all" onFilterChange={vi.fn()} />,
    );
    expect(screen.getByText("All")).toBeInTheDocument();
    expect(screen.getByText("Lead")).toBeInTheDocument();
  });

  it("renders teammate tabs with names", () => {
    const teammates = [
      makeTeammate({ name: "coder-1" }),
      makeTeammate({ name: "coder-2", color: "#10b981" }),
    ];
    render(
      <TeamFilterTabs teammates={teammates} activeFilter="all" onFilterChange={vi.fn()} />,
    );
    expect(screen.getByText("coder-1")).toBeInTheDocument();
    expect(screen.getByText("coder-2")).toBeInTheDocument();
  });

  it("calls onFilterChange with 'all' when All clicked", () => {
    const onChange = vi.fn();
    render(
      <TeamFilterTabs teammates={[]} activeFilter="lead" onFilterChange={onChange} />,
    );
    fireEvent.click(screen.getByText("All"));
    expect(onChange).toHaveBeenCalledWith("all");
  });

  it("calls onFilterChange with 'lead' when Lead clicked", () => {
    const onChange = vi.fn();
    render(
      <TeamFilterTabs teammates={[]} activeFilter="all" onFilterChange={onChange} />,
    );
    fireEvent.click(screen.getByText("Lead"));
    expect(onChange).toHaveBeenCalledWith("lead");
  });

  it("calls onFilterChange with teammate name when teammate clicked", () => {
    const onChange = vi.fn();
    const teammates = [makeTeammate({ name: "coder-1" })];
    render(
      <TeamFilterTabs teammates={teammates} activeFilter="all" onFilterChange={onChange} />,
    );
    fireEvent.click(screen.getByText("coder-1"));
    expect(onChange).toHaveBeenCalledWith("coder-1");
  });

  it("renders color dot for teammate tabs", () => {
    const teammates = [makeTeammate({ name: "coder-1", color: "#3b82f6" })];
    const { container } = render(
      <TeamFilterTabs teammates={teammates} activeFilter="all" onFilterChange={vi.fn()} />,
    );
    const dots = container.querySelectorAll("span.rounded-full");
    expect(dots.length).toBeGreaterThanOrEqual(1);
  });
});
