import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PlanDisplay } from "./PlanDisplay";
import type { Artifact } from "@/types/artifact";

const mockPlan: Artifact = {
  id: "artifact-1",
  type: "specification",
  name: "Authentication Implementation Plan",
  content: {
    type: "inline",
    text: `# Authentication Plan\n\n## Overview\nImplement JWT-based authentication system.`,
  },
  metadata: {
    createdAt: "2026-01-26T10:00:00Z",
    createdBy: "orchestrator-ideation",
    version: 1,
  },
  derivedFrom: [],
  bucketId: "prd-library",
};

describe("PlanDisplay", () => {
  it("renders plan header and starts collapsed", () => {
    render(<PlanDisplay plan={mockPlan} />);

    expect(screen.getByText("Authentication Implementation Plan")).toBeInTheDocument();
    expect(screen.queryByText("Authentication Plan")).not.toBeInTheDocument();
  });

  it("expands and renders markdown content", () => {
    render(<PlanDisplay plan={mockPlan} />);

    fireEvent.click(screen.getByRole("button", { name: /Authentication Implementation Plan/i }));

    const heading = screen.getByText("Authentication Plan");
    expect(heading).toBeInTheDocument();
    expect(heading.tagName).toBe("H1");
    expect(screen.getByText(/JWT-based authentication/i)).toBeInTheDocument();
  });

  it("shows linked proposal counts", () => {
    const { rerender } = render(<PlanDisplay plan={mockPlan} linkedProposalsCount={3} />);
    expect(screen.getByText("3 linked proposals")).toBeInTheDocument();

    rerender(<PlanDisplay plan={mockPlan} linkedProposalsCount={1} />);
    expect(screen.getByText("1 linked proposal")).toBeInTheDocument();
  });

  it("calls onEdit and onExport from action buttons", () => {
    const onEdit = vi.fn();
    const onExport = vi.fn();
    const { container } = render(<PlanDisplay plan={mockPlan} onEdit={onEdit} onExport={onExport} />);

    const buttons = container.querySelectorAll("button");
    fireEvent.click(buttons[1]);
    fireEvent.click(buttons[2]);

    expect(onEdit).toHaveBeenCalledTimes(1);
    expect(onExport).toHaveBeenCalledTimes(1);
  });

  it("shows and handles Approve action", () => {
    const onApprove = vi.fn();
    render(<PlanDisplay plan={mockPlan} showApprove={true} isApproved={false} onApprove={onApprove} />);

    fireEvent.click(screen.getByRole("button", { name: /approve/i }));
    expect(onApprove).toHaveBeenCalledTimes(1);
  });

  it("shows approved badge when already approved", () => {
    render(<PlanDisplay plan={mockPlan} showApprove={true} isApproved={true} />);

    expect(screen.getByText("Approved")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /approve/i })).not.toBeInTheDocument();
  });

  it("shows no content for empty inline text", () => {
    const emptyPlan: Artifact = {
      ...mockPlan,
      content: { type: "inline", text: "" },
    };

    render(<PlanDisplay plan={emptyPlan} isExpanded={true} />);
    expect(screen.getByText("No content available")).toBeInTheDocument();
  });

  it("shows no content for file artifacts", () => {
    const filePlan: Artifact = {
      ...mockPlan,
      content: { type: "file", path: "/path/to/plan.md" },
    };

    render(<PlanDisplay plan={filePlan} isExpanded={true} />);
    expect(screen.getByText("No content available")).toBeInTheDocument();
  });
});
