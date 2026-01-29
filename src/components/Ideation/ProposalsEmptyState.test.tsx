import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { ProposalsEmptyState } from "./ProposalsEmptyState";

describe("ProposalsEmptyState", () => {
  it("renders the empty state container", () => {
    render(<ProposalsEmptyState />);
    expect(screen.getByTestId("proposals-empty-state")).toBeInTheDocument();
  });

  it("displays 'No proposals yet' heading", () => {
    render(<ProposalsEmptyState />);
    expect(screen.getByText("No proposals yet")).toBeInTheDocument();
  });

  it("displays the 'From chat' hint", () => {
    render(<ProposalsEmptyState />);
    expect(screen.getByText("From chat")).toBeInTheDocument();
  });

  it("displays the 'or' divider", () => {
    render(<ProposalsEmptyState />);
    expect(screen.getByText("or")).toBeInTheDocument();
  });

  it("displays the drop hint", () => {
    render(<ProposalsEmptyState />);
    expect(screen.getByTestId("drop-hint")).toBeInTheDocument();
    expect(
      screen.getByText(/Drag a markdown file here/i)
    ).toBeInTheDocument();
    expect(screen.getByText(/to import a plan/i)).toBeInTheDocument();
  });

  it("renders the FileDown icon in drop hint", () => {
    render(<ProposalsEmptyState />);
    const dropHint = screen.getByTestId("drop-hint");
    const svgIcon = dropHint.querySelector("svg");
    expect(svgIcon).toBeInTheDocument();
  });

  it("renders the Lightbulb icon", () => {
    render(<ProposalsEmptyState />);
    // The lightbulb is in the central icon container
    const container = screen.getByTestId("proposals-empty-state");
    const svgs = container.querySelectorAll("svg");
    // Should have at least 3 SVGs: arrow, FileDown, and Lightbulb
    expect(svgs.length).toBeGreaterThanOrEqual(3);
  });
});
