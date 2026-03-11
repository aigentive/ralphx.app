import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { AutoVerificationCard } from "./AutoVerificationCard";

describe("AutoVerificationCard", () => {
  const defaultProps = {
    content: "Please verify this implementation.",
    createdAt: "2026-03-12T10:00:00Z",
  };

  it("renders collapsed by default showing Auto-verification label", () => {
    render(<AutoVerificationCard {...defaultProps} />);
    expect(screen.getByText("Auto-verification")).toBeInTheDocument();
    // Content should NOT be visible while collapsed
    expect(screen.queryByText(defaultProps.content)).not.toBeInTheDocument();
  });

  it("expands on click to show content", () => {
    render(<AutoVerificationCard {...defaultProps} />);
    const button = screen.getByRole("button");
    fireEvent.click(button);
    expect(screen.getByText(defaultProps.content)).toBeInTheDocument();
  });

  it("re-collapses on second click", () => {
    render(<AutoVerificationCard {...defaultProps} />);
    const button = screen.getByRole("button");
    fireEvent.click(button);
    expect(screen.getByText(defaultProps.content)).toBeInTheDocument();
    fireEvent.click(button);
    expect(screen.queryByText(defaultProps.content)).not.toBeInTheDocument();
  });

  it("strips <auto-verification> wrapper tags from content", () => {
    const wrappedContent = "<auto-verification>\nCheck this code.\n</auto-verification>";
    render(<AutoVerificationCard content={wrappedContent} createdAt={defaultProps.createdAt} />);
    const button = screen.getByRole("button");
    fireEvent.click(button);
    expect(screen.getByText("Check this code.")).toBeInTheDocument();
    // Should not show the tags
    expect(screen.queryByText(/<auto-verification>/)).not.toBeInTheDocument();
  });

  it("strips case-insensitive <Auto-Verification> wrapper tags", () => {
    const wrappedContent = "<Auto-Verification>  Verify this.  </Auto-Verification>";
    render(<AutoVerificationCard content={wrappedContent} createdAt={defaultProps.createdAt} />);
    const button = screen.getByRole("button");
    fireEvent.click(button);
    expect(screen.getByText("Verify this.")).toBeInTheDocument();
  });
});
