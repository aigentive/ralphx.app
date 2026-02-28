/**
 * AnalysisBanner.test.tsx
 * Tests for the banner shown during dependency analysis
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { AnalysisBanner } from "./PlanningView";

describe("AnalysisBanner", () => {
  it("renders with data-testid", () => {
    render(<AnalysisBanner />);
    expect(screen.getByTestId("analysis-banner")).toBeInTheDocument();
  });

  it("shows analyzing text", () => {
    render(<AnalysisBanner />);
    expect(
      screen.getByText(/Analyzing dependencies/i)
    ).toBeInTheDocument();
  });

  it("mentions accept availability", () => {
    render(<AnalysisBanner />);
    expect(
      screen.getByText(/accept will be available when complete/i)
    ).toBeInTheDocument();
  });

  it("matches snapshot", () => {
    const { container } = render(<AnalysisBanner />);
    expect(container.firstChild).toMatchSnapshot();
  });
});
