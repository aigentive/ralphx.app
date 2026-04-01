/**
 * TeamSystemEvent tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeamSystemEvent } from "./TeamSystemEvent";

describe("TeamSystemEvent", () => {
  it("renders the event message", () => {
    render(<TeamSystemEvent message="coder-3 joined the team" />);
    expect(screen.getByText("coder-3 joined the team")).toBeInTheDocument();
  });

  it("renders timestamp when provided", () => {
    render(
      <TeamSystemEvent
        message="Wave 2 validated"
        timestamp="2026-02-15T14:30:00Z"
      />
    );
    expect(screen.getByText("Wave 2 validated")).toBeInTheDocument();
  });

  it("does not render timestamp when not provided", () => {
    const { container } = render(<TeamSystemEvent message="test" />);
    // Only the message text, no time span
    const spans = container.querySelectorAll("span");
    expect(spans.length).toBe(0);
  });
});
