/**
 * Tests for TaskBoardSkeleton component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TaskBoardSkeleton } from "./TaskBoardSkeleton";

describe("TaskBoardSkeleton", () => {
  it("should render with data-testid", () => {
    render(<TaskBoardSkeleton />);
    expect(screen.getByTestId("task-board-skeleton")).toBeInTheDocument();
  });

  it("should render 7 column placeholders", () => {
    render(<TaskBoardSkeleton />);
    const columns = screen.getAllByTestId(/skeleton-column-/);
    expect(columns).toHaveLength(7);
  });

  it("should render column headers", () => {
    render(<TaskBoardSkeleton />);
    const headers = screen.getAllByTestId(/skeleton-header-/);
    expect(headers).toHaveLength(7);
  });

  it("should render card placeholders in each column", () => {
    render(<TaskBoardSkeleton />);
    // Each column should have some card placeholders
    const cards = screen.getAllByTestId(/skeleton-card-/);
    expect(cards.length).toBeGreaterThan(0);
  });

  it("should apply animate-pulse class for loading animation", () => {
    render(<TaskBoardSkeleton />);
    const skeleton = screen.getByTestId("task-board-skeleton");
    // Check that there are elements with pulse animation
    const pulsingElements = skeleton.querySelectorAll(".animate-pulse");
    expect(pulsingElements.length).toBeGreaterThan(0);
  });

  it("should use design system background colors via CSS variables", () => {
    render(<TaskBoardSkeleton />);
    const skeleton = screen.getByTestId("task-board-skeleton");
    // Check that it uses CSS variables for backgrounds
    expect(skeleton.style.backgroundColor).toBe("var(--bg-base)");
    const column = screen.getByTestId("skeleton-column-0");
    expect(column.style.backgroundColor).toBe("var(--bg-surface)");
  });
});
