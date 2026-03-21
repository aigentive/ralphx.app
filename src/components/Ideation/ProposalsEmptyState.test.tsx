import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
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

  it("arrow points RIGHT — SVG path is M2 7h10 not M12 7H2", () => {
    render(<ProposalsEmptyState />);
    const container = screen.getByTestId("proposals-empty-state");
    const paths = container.querySelectorAll("path");
    const arrowPath = Array.from(paths).find((p) =>
      p.getAttribute("d")?.startsWith("M2 7h10")
    );
    expect(arrowPath).toBeTruthy();
    // Confirm no left-pointing arrow remains
    const leftArrow = Array.from(paths).find((p) =>
      p.getAttribute("d")?.startsWith("M12 7H2")
    );
    expect(leftArrow).toBeUndefined();
  });

  it("renders the Lightbulb icon", () => {
    render(<ProposalsEmptyState />);
    const container = screen.getByTestId("proposals-empty-state");
    const svgs = container.querySelectorAll("svg");
    // Should have at least 2 SVGs: Lightbulb + arrow hint
    expect(svgs.length).toBeGreaterThanOrEqual(2);
  });

  it("does not show drop-hint when onBrowse is not provided", () => {
    render(<ProposalsEmptyState />);
    expect(screen.queryByTestId("drop-hint")).not.toBeInTheDocument();
  });

  it("displays the 'or' divider and drop hint when onBrowse is provided", () => {
    render(<ProposalsEmptyState onBrowse={() => {}} />);
    expect(screen.getByText("or")).toBeInTheDocument();
    expect(screen.getByTestId("drop-hint")).toBeInTheDocument();
    expect(screen.getByText(/Drag a markdown file here/i)).toBeInTheDocument();
    expect(screen.getByText(/click to browse/i)).toBeInTheDocument();
  });

  it("calls onBrowse when drop hint is clicked", async () => {
    const onBrowse = vi.fn();
    render(<ProposalsEmptyState onBrowse={onBrowse} />);
    await userEvent.click(screen.getByTestId("drop-hint"));
    expect(onBrowse).toHaveBeenCalledTimes(1);
  });

  it("renders FileDown icon in drop hint when onBrowse is provided", () => {
    render(<ProposalsEmptyState onBrowse={() => {}} />);
    const dropHint = screen.getByTestId("drop-hint");
    const svgIcon = dropHint.querySelector("svg");
    expect(svgIcon).toBeInTheDocument();
  });

  it("renders at least 3 SVGs when onBrowse is provided (arrow + FileDown + Lightbulb)", () => {
    render(<ProposalsEmptyState onBrowse={() => {}} />);
    const container = screen.getByTestId("proposals-empty-state");
    const svgs = container.querySelectorAll("svg");
    expect(svgs.length).toBeGreaterThanOrEqual(3);
  });
});
