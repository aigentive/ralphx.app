/**
 * Tests for SessionGroupSkeleton component
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SessionGroupSkeleton } from "./SessionGroupSkeleton";

describe("SessionGroupSkeleton", () => {
  it("should render with data-testid", () => {
    render(<SessionGroupSkeleton />);
    expect(screen.getByTestId("session-group-skeleton")).toBeInTheDocument();
  });

  it("should render 4 skeleton items by default", () => {
    render(<SessionGroupSkeleton />);
    const skeleton = screen.getByTestId("session-group-skeleton");
    // Each item is a direct child div
    expect(skeleton.children).toHaveLength(4);
  });

  it("should render correct count when count prop is provided", () => {
    render(<SessionGroupSkeleton count={3} />);
    const skeleton = screen.getByTestId("session-group-skeleton");
    expect(skeleton.children).toHaveLength(3);
  });

  it("should apply animate-pulse class for loading animation", () => {
    render(<SessionGroupSkeleton />);
    const skeleton = screen.getByTestId("session-group-skeleton");
    const pulsingElements = skeleton.querySelectorAll(".animate-pulse");
    expect(pulsingElements.length).toBeGreaterThan(0);
  });

  it("should render with count=1 minimum", () => {
    render(<SessionGroupSkeleton count={1} />);
    const skeleton = screen.getByTestId("session-group-skeleton");
    expect(skeleton.children).toHaveLength(1);
  });
});
