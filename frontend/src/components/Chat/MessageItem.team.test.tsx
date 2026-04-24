/**
 * MessageItem team extension tests — teammate name badge and color border
 */

import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TooltipProvider } from "@/components/ui/tooltip";
import { MessageItem } from "./MessageItem";

function renderMessageItem(ui: React.ReactElement) {
  return render(<TooltipProvider delayDuration={0}>{ui}</TooltipProvider>);
}

describe("MessageItem — team extensions", () => {
  it("renders Bot icon for assistant without teammateName", () => {
    const { container } = renderMessageItem(
      <MessageItem role="assistant" content="Hello" createdAt="2026-02-15T10:00:00Z" />,
    );
    // Bot icon is an SVG with lucide class
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });

  it("renders teammate name badge instead of Bot icon", () => {
    const { container } = renderMessageItem(
      <MessageItem
        role="assistant"
        content="Done with auth"
        createdAt="2026-02-15T10:00:00Z"
        teammateName="coder-1"
        teammateColor="#3b82f6"
      />,
    );
    expect(screen.getByText("coder-1")).toBeInTheDocument();
    // Bot icon should NOT be present
    const bots = container.querySelectorAll("svg.lucide-bot");
    expect(bots.length).toBe(0);
  });

  it("renders color border when teammateColor is provided", () => {
    const { container } = renderMessageItem(
      <MessageItem
        role="assistant"
        content="Hello"
        createdAt="2026-02-15T10:00:00Z"
        teammateName="coder-1"
        teammateColor="#3b82f6"
      />,
    );
    const wrapper = container.firstChild as HTMLElement;
    // Browser normalizes hex to rgb
    expect(wrapper.style.borderLeft).toBe("2px solid rgb(59, 130, 246)");
    expect(wrapper.style.paddingLeft).toBe("8px");
  });

  it("does not render color border without teammateColor", () => {
    const { container } = renderMessageItem(
      <MessageItem role="assistant" content="Hello" createdAt="2026-02-15T10:00:00Z" />,
    );
    const wrapper = container.firstChild as HTMLElement;
    expect(wrapper.style.borderLeft).toBe("");
  });

  it("renders teammate color dot", () => {
    const { container } = renderMessageItem(
      <MessageItem
        role="assistant"
        content="Hello"
        createdAt="2026-02-15T10:00:00Z"
        teammateName="coder-1"
        teammateColor="#3b82f6"
      />,
    );
    const dot = container.querySelector("span.rounded-full");
    expect(dot).toBeInTheDocument();
    expect(dot).toHaveStyle({ backgroundColor: "#3b82f6" });
  });

  it("does not show teammate badge for user messages", () => {
    renderMessageItem(
      <MessageItem
        role="user"
        content="Hello"
        createdAt="2026-02-15T10:00:00Z"
        teammateName="coder-1"
        teammateColor="#3b82f6"
      />,
    );
    // teammateName badge only shows for non-user messages
    expect(screen.queryByText("coder-1")).not.toBeInTheDocument();
  });
});
