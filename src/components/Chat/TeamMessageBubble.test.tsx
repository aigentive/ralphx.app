/**
 * TeamMessageBubble tests
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TeamMessageBubble } from "./TeamMessageBubble";

describe("TeamMessageBubble", () => {
  it("renders from, to, and content", () => {
    render(
      <TeamMessageBubble
        from="coder-1"
        to="coder-2"
        content="Session type is in auth.ts"
      />
    );
    expect(screen.getByText("coder-1")).toBeInTheDocument();
    expect(screen.getByText("coder-2")).toBeInTheDocument();
    expect(screen.getByText("Session type is in auth.ts")).toBeInTheDocument();
  });

  it("renders arrow separator", () => {
    render(
      <TeamMessageBubble from="a" to="b" content="test" />
    );
    expect(screen.getByText("→")).toBeInTheDocument();
  });

  it("renders color dot when fromColor is provided", () => {
    const { container } = render(
      <TeamMessageBubble from="coder-1" to="coder-2" content="test" fromColor="#3b82f6" />
    );
    const dot = container.querySelector("span.rounded-full");
    expect(dot).toBeInTheDocument();
    expect(dot).toHaveStyle({ backgroundColor: "#3b82f6" });
  });

  it("does not render color dot when fromColor is not provided", () => {
    const { container } = render(
      <TeamMessageBubble from="coder-1" to="coder-2" content="test" />
    );
    const dots = container.querySelectorAll("span.rounded-full");
    expect(dots.length).toBe(0);
  });
});
