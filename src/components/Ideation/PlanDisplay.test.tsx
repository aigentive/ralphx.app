/**
 * Tests for PlanDisplay component
 */

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { PlanDisplay } from "./PlanDisplay";
import type { Artifact } from "@/types/artifact";

// ============================================================================
// Test Data
// ============================================================================

const mockPlan: Artifact = {
  id: "artifact-1",
  type: "specification",
  name: "Authentication Implementation Plan",
  content: {
    type: "inline",
    text: `# Authentication Plan

## Overview
Implement JWT-based authentication system.

## Components
- Login form
- Token storage
- Protected routes

## Steps
1. Create login API endpoint
2. Implement token storage
3. Add route guards`,
  },
  metadata: {
    createdAt: "2026-01-26T10:00:00Z",
    createdBy: "orchestrator-ideation",
    version: 1,
  },
  derivedFrom: [],
  bucketId: "prd-library",
};

// ============================================================================
// Tests
// ============================================================================

describe("PlanDisplay", () => {
  it("renders plan name and content", () => {
    render(<PlanDisplay plan={mockPlan} />);

    expect(screen.getByText("Authentication Implementation Plan")).toBeInTheDocument();
    expect(screen.getByText(/Authentication Plan/i)).toBeInTheDocument();
    expect(screen.getByText(/JWT-based authentication/i)).toBeInTheDocument();
  });

  it("shows linked proposals count when provided", () => {
    render(<PlanDisplay plan={mockPlan} linkedProposalsCount={3} />);

    expect(screen.getByText("3 proposals linked")).toBeInTheDocument();
  });

  it("shows singular 'proposal' for count of 1", () => {
    render(<PlanDisplay plan={mockPlan} linkedProposalsCount={1} />);

    expect(screen.getByText("1 proposal linked")).toBeInTheDocument();
  });

  it("renders Edit and Export buttons", () => {
    render(<PlanDisplay plan={mockPlan} />);

    expect(screen.getByRole("button", { name: /edit/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /export/i })).toBeInTheDocument();
  });

  it("calls onEdit when Edit button is clicked", () => {
    const onEdit = vi.fn();
    render(<PlanDisplay plan={mockPlan} onEdit={onEdit} />);

    const editButton = screen.getByRole("button", { name: /edit/i });
    fireEvent.click(editButton);

    expect(onEdit).toHaveBeenCalledTimes(1);
  });

  it("calls onExport when Export button is clicked", () => {
    const onExport = vi.fn();
    render(<PlanDisplay plan={mockPlan} onExport={onExport} />);

    const exportButton = screen.getByRole("button", { name: /export/i });
    fireEvent.click(exportButton);

    expect(onExport).toHaveBeenCalledTimes(1);
  });

  it("shows Approve button when showApprove is true and not approved", () => {
    render(<PlanDisplay plan={mockPlan} showApprove={true} isApproved={false} />);

    expect(screen.getByRole("button", { name: /approve plan/i })).toBeInTheDocument();
  });

  it("does not show Approve button when showApprove is false", () => {
    render(<PlanDisplay plan={mockPlan} showApprove={false} />);

    expect(screen.queryByRole("button", { name: /approve plan/i })).not.toBeInTheDocument();
  });

  it("shows Approved badge when isApproved is true", () => {
    render(<PlanDisplay plan={mockPlan} showApprove={true} isApproved={true} />);

    expect(screen.getByText("Approved")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /approve plan/i })).not.toBeInTheDocument();
  });

  it("calls onApprove when Approve button is clicked", () => {
    const onApprove = vi.fn();
    render(<PlanDisplay plan={mockPlan} showApprove={true} onApprove={onApprove} />);

    const approveButton = screen.getByRole("button", { name: /approve plan/i });
    fireEvent.click(approveButton);

    expect(onApprove).toHaveBeenCalledTimes(1);
  });

  it("can be collapsed and expanded", () => {
    const { container } = render(<PlanDisplay plan={mockPlan} />);

    // Content should be visible initially
    expect(screen.getByText(/JWT-based authentication/i)).toBeInTheDocument();

    // Find the collapsible content container
    const collapsibleContent = container.querySelector("[data-slot='collapsible-content']");
    expect(collapsibleContent).toHaveAttribute("data-state", "open");

    // Find and click the collapse button
    const collapseButton = screen.getAllByRole("button")[0]; // First button is the collapse trigger
    fireEvent.click(collapseButton);

    // Content should be hidden (checking via data-state attribute)
    expect(collapsibleContent).toHaveAttribute("data-state", "closed");
  });

  it("handles plan with no content gracefully", () => {
    const emptyPlan: Artifact = {
      ...mockPlan,
      content: {
        type: "inline",
        text: "",
      },
    };

    render(<PlanDisplay plan={emptyPlan} />);

    expect(screen.getByText("No content")).toBeInTheDocument();
  });

  it("handles file-type content by showing empty", () => {
    const filePlan: Artifact = {
      ...mockPlan,
      content: {
        type: "file",
        path: "/path/to/plan.md",
      },
    };

    render(<PlanDisplay plan={filePlan} />);

    // File content is not rendered, should show "No content"
    expect(screen.getByText("No content")).toBeInTheDocument();
  });

  it("renders markdown with proper formatting", () => {
    render(<PlanDisplay plan={mockPlan} />);

    // Check for heading rendering
    const heading = screen.getByText("Authentication Plan");
    expect(heading.tagName).toBe("H1");

    // Check for subheadings
    expect(screen.getByText("Overview")).toBeInTheDocument();
    expect(screen.getByText("Components")).toBeInTheDocument();
    expect(screen.getByText("Steps")).toBeInTheDocument();
  });

  it("exports plan as markdown file when export clicked without custom handler", () => {
    // Mock URL.createObjectURL and document methods
    const createObjectURL = vi.fn(() => "blob:mock-url");
    const revokeObjectURL = vi.fn();
    global.URL.createObjectURL = createObjectURL;
    global.URL.revokeObjectURL = revokeObjectURL;

    const appendChild = vi.spyOn(document.body, "appendChild");
    const removeChild = vi.spyOn(document.body, "removeChild");

    render(<PlanDisplay plan={mockPlan} />);

    const exportButton = screen.getByRole("button", { name: /export/i });
    fireEvent.click(exportButton);

    expect(createObjectURL).toHaveBeenCalledWith(expect.any(Blob));
    expect(appendChild).toHaveBeenCalled();
    expect(removeChild).toHaveBeenCalled();
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:mock-url");

    // Cleanup
    appendChild.mockRestore();
    removeChild.mockRestore();
  });

  it("applies correct styling classes for premium design", () => {
    const { container } = render(<PlanDisplay plan={mockPlan} />);

    // Check for Card with border and background
    const card = container.querySelector(".border-\\[var\\(--border-primary\\)\\]");
    expect(card).toBeInTheDocument();

    // Check for proper text color classes
    expect(container.querySelector(".text-\\[var\\(--text-primary\\)\\]")).toBeInTheDocument();
  });
});
