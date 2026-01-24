import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

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
});
