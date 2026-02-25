import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { PaneStream } from "./PaneStream";

describe("PaneStream", () => {
  it("shows redirect message with teammate name", () => {
    render(<PaneStream contextKey="test" teammateName="worker-1" />);
    expect(screen.getByText(/worker-1/)).toBeInTheDocument();
    expect(screen.getByText(/teammate tab/)).toBeInTheDocument();
  });
});
