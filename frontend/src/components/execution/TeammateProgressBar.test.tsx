/**
 * TeammateProgressBar component tests
 *
 * Tests progress percentage calculation and div-by-zero guard.
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeammateProgressBar } from "./TeammateProgressBar";

describe("TeammateProgressBar", () => {
  it("calculates and displays correct percentage", () => {
    render(<TeammateProgressBar completed={3} total={10} />);
    expect(screen.getByText("30%")).toBeInTheDocument();
  });

  it("rounds percentage to nearest integer", () => {
    render(<TeammateProgressBar completed={1} total={3} />);
    // 1/3 = 33.33... → rounds to 33%
    expect(screen.getByText("33%")).toBeInTheDocument();
  });

  it("shows 100% when all steps completed", () => {
    render(<TeammateProgressBar completed={5} total={5} />);
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("guards against division by zero (total=0 → 0%)", () => {
    render(<TeammateProgressBar completed={0} total={0} />);
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  it("sets fill bar width matching percentage", () => {
    const { container } = render(<TeammateProgressBar completed={7} total={10} />);
    // The fill bar is the inner div with transition-[width]
    const fillBar = container.querySelector(".transition-\\[width\\]");
    expect(fillBar).toHaveStyle({ width: "70%" });
  });
});
