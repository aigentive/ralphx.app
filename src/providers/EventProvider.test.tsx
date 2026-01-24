import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { EventProvider } from "./EventProvider";

// Mock the event hooks
vi.mock("@/hooks/useEvents", () => ({
  useTaskEvents: vi.fn(),
  useSupervisorAlerts: vi.fn(),
  useReviewEvents: vi.fn(),
  useFileChangeEvents: vi.fn(),
}));

import {
  useTaskEvents,
  useSupervisorAlerts,
  useReviewEvents,
  useFileChangeEvents,
} from "@/hooks/useEvents";

describe("EventProvider", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should render children", () => {
    render(
      <EventProvider>
        <div data-testid="child">Hello World</div>
      </EventProvider>
    );

    expect(screen.getByTestId("child")).toHaveTextContent("Hello World");
  });

  it("should call useTaskEvents hook", () => {
    render(
      <EventProvider>
        <div>Test</div>
      </EventProvider>
    );

    expect(useTaskEvents).toHaveBeenCalled();
  });

  it("should call useSupervisorAlerts hook", () => {
    render(
      <EventProvider>
        <div>Test</div>
      </EventProvider>
    );

    expect(useSupervisorAlerts).toHaveBeenCalled();
  });

  it("should call useReviewEvents hook", () => {
    render(
      <EventProvider>
        <div>Test</div>
      </EventProvider>
    );

    expect(useReviewEvents).toHaveBeenCalled();
  });

  it("should call useFileChangeEvents hook", () => {
    render(
      <EventProvider>
        <div>Test</div>
      </EventProvider>
    );

    expect(useFileChangeEvents).toHaveBeenCalled();
  });

  it("should render multiple children", () => {
    render(
      <EventProvider>
        <div data-testid="child1">First</div>
        <div data-testid="child2">Second</div>
      </EventProvider>
    );

    expect(screen.getByTestId("child1")).toBeInTheDocument();
    expect(screen.getByTestId("child2")).toBeInTheDocument();
  });

  it("should render nested components", () => {
    render(
      <EventProvider>
        <div data-testid="outer">
          <span data-testid="inner">Nested content</span>
        </div>
      </EventProvider>
    );

    expect(screen.getByTestId("outer")).toContainElement(
      screen.getByTestId("inner")
    );
  });
});
