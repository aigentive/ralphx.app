import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import { PlanEmptyState } from "./PlanEmptyState";

describe("PlanEmptyState", () => {
  it("renders heading 'No plan yet'", () => {
    render(<PlanEmptyState />);
    expect(screen.getByText("No plan yet")).toBeInTheDocument();
  });

  it("renders the description text", () => {
    render(<PlanEmptyState />);
    expect(
      screen.getByText(
        "The implementation plan will appear here when created from the conversation"
      )
    ).toBeInTheDocument();
  });

  it("has data-testid='plan-empty-state'", () => {
    render(<PlanEmptyState />);
    expect(screen.getByTestId("plan-empty-state")).toBeInTheDocument();
  });

  it("renders document mock visual", () => {
    render(<PlanEmptyState />);
    expect(screen.getByTestId("plan-document-mock")).toBeInTheDocument();
  });

  it("does not render browse button when onBrowse not provided", () => {
    render(<PlanEmptyState />);
    expect(screen.queryByTestId("drop-hint")).toBeNull();
  });

  it("threads onBrowse through to browse button", async () => {
    const onBrowse = vi.fn();
    render(<PlanEmptyState onBrowse={onBrowse} />);
    await userEvent.click(screen.getByTestId("drop-hint"));
    expect(onBrowse).toHaveBeenCalledTimes(1);
  });
});
