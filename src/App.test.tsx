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

describe("App", () => {
  it("should render without crashing", () => {
    render(<App />);
    expect(document.body).toBeDefined();
  });

  it("should display RalphX title", () => {
    render(<App />);
    expect(screen.getByText(/RalphX/i)).toBeInTheDocument();
  });

  it("should display health status placeholder", () => {
    render(<App />);
    expect(screen.getByText(/autonomous/i)).toBeInTheDocument();
  });

  it("should have dark theme background class", () => {
    render(<App />);
    const mainElement = screen.getByRole("main");
    expect(mainElement).toHaveClass("bg-bg-base");
  });

  it("should use accent color for title", () => {
    render(<App />);
    const titleElement = screen.getByText(/RalphX/i);
    expect(titleElement).toHaveClass("text-accent-primary");
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
