import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import { TabEmptyState } from "./TabEmptyState";

const TestIcon = () => <div data-testid="test-icon" />;

describe("TabEmptyState", () => {
  it("renders heading and description", () => {
    render(
      <TabEmptyState
        icon={<TestIcon />}
        heading="Test heading"
        description="Test description"
      />
    );
    expect(screen.getByText("Test heading")).toBeInTheDocument();
    expect(screen.getByText("Test description")).toBeInTheDocument();
  });

  it("renders the icon slot", () => {
    render(
      <TabEmptyState
        icon={<TestIcon />}
        heading="Heading"
        description="Desc"
      />
    );
    expect(screen.getByTestId("test-icon")).toBeInTheDocument();
  });

  it("SVG arrow path points RIGHT (d attribute equals 'M2 7h10m0 0l-3-3m3 3l-3 3')", () => {
    const { container } = render(
      <TabEmptyState
        icon={<TestIcon />}
        heading="Heading"
        description="Desc"
      />
    );
    const paths = container.querySelectorAll("path");
    const arrowPath = Array.from(paths).find(
      (p) => p.getAttribute("d") === "M2 7h10m0 0l-3-3m3 3l-3 3"
    );
    expect(arrowPath).toBeDefined();
  });

  it("hides browse button and 'or' divider when onBrowse not provided", () => {
    render(
      <TabEmptyState
        icon={<TestIcon />}
        heading="Heading"
        description="Desc"
      />
    );
    expect(screen.queryByTestId("drop-hint")).toBeNull();
    expect(screen.queryByText("or")).toBeNull();
  });

  it("shows browse button when onBrowse is provided", () => {
    const onBrowse = vi.fn();
    render(
      <TabEmptyState
        icon={<TestIcon />}
        heading="Heading"
        description="Desc"
        onBrowse={onBrowse}
      />
    );
    expect(screen.getByTestId("drop-hint")).toBeInTheDocument();
    expect(screen.getByText("or")).toBeInTheDocument();
  });

  it("fires onBrowse when browse button is clicked", async () => {
    const onBrowse = vi.fn();
    render(
      <TabEmptyState
        icon={<TestIcon />}
        heading="Heading"
        description="Desc"
        onBrowse={onBrowse}
      />
    );
    await userEvent.click(screen.getByTestId("drop-hint"));
    expect(onBrowse).toHaveBeenCalledTimes(1);
  });
});
