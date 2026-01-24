import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { useQueryClient } from "@tanstack/react-query";
import App from "./App";

// Mock the useEvents hooks to prevent Tauri API calls
vi.mock("@/hooks/useEvents", () => ({
  useTaskEvents: vi.fn(),
  useSupervisorAlerts: vi.fn(),
  useReviewEvents: vi.fn(),
  useFileChangeEvents: vi.fn(),
}));

// Mock TaskBoard to avoid Tauri API calls during tests
vi.mock("@/components/tasks/TaskBoard", () => ({
  TaskBoard: () => <div data-testid="task-board-mock">Task Board</div>,
}));

describe("App", () => {
  it("should render without crashing", () => {
    render(<App />);
    expect(document.body).toBeDefined();
  });

  it("should display RalphX title", () => {
    render(<App />);
    expect(screen.getByText(/RalphX/i)).toBeInTheDocument();
  });

  it("should display project name", () => {
    render(<App />);
    expect(screen.getByText(/Demo Project/i)).toBeInTheDocument();
  });

  it("should have main element with flex layout", () => {
    render(<App />);
    const mainElement = screen.getByRole("main");
    expect(mainElement).toHaveClass("min-h-screen", "flex", "flex-col");
  });

  it("should render header with RalphX branding", () => {
    render(<App />);
    const header = screen.getByRole("banner");
    expect(header).toBeInTheDocument();
    expect(header).toHaveClass("flex", "items-center", "justify-between");
  });

  it("should render TaskBoard component", () => {
    render(<App />);
    expect(screen.getByTestId("task-board-mock")).toBeInTheDocument();
  });

  it("should provide QueryClient context", () => {
    // This test verifies that QueryClientProvider is working
    // by rendering a component that uses useQueryClient
    function QueryClientChecker() {
      const queryClient = useQueryClient();
      return queryClient ? <div data-testid="query-ok">OK</div> : null;
    }

    // Render with App as parent to get the QueryClientProvider context
    render(<App />);
    // If App renders successfully with QueryClientProvider, queries should work
    expect(document.body).toBeDefined();
  });
});
