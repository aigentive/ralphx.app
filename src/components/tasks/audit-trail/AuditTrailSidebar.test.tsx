import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { AuditTrailSidebar, type AuditPhase } from "./AuditTrailSidebar";

const NOW = 1740268800000; // 2026-02-23T00:00:00Z (fixed)

const phases: AuditPhase[] = [
  {
    id: "phase-exec-1",
    label: "Execution #1",
    type: "execution",
    status: "executing",
    startTime: NOW - 900000, // 15 min earlier
    endTime: NOW - 840000,   // 14 min earlier → 1m duration
    entryCount: 12,
  },
  {
    id: "phase-review-1",
    label: "Review #1",
    type: "review",
    status: "approved",
    startTime: NOW - 840000,
    endTime: NOW - 794000, // ~46s later
    entryCount: 3,
    reviewOutcome: "approved",
  },
  {
    id: "phase-merge-1",
    label: "Merge",
    type: "merge",
    status: "merged",
    startTime: NOW - 794000,
    endTime: NOW - 782000, // 12s later
    entryCount: 2,
  },
];

const defaultProps = {
  phases,
  selectedPhaseId: null as string | null,
  onPhaseSelect: vi.fn(),
  totalEvents: 47,
  dateRange: "Feb 23, 2026",
  isLoading: false,
};

describe("AuditTrailSidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders phase groups with correct labels", () => {
    render(<AuditTrailSidebar {...defaultProps} />);
    expect(screen.getByText("Execution #1")).toBeInTheDocument();
    expect(screen.getByText("Review #1")).toBeInTheDocument();
    expect(screen.getByText("Merge")).toBeInTheDocument();
  });

  it("clicking a phase calls onPhaseSelect with the phase ID", () => {
    render(<AuditTrailSidebar {...defaultProps} />);
    fireEvent.click(screen.getByTestId("phase-button-phase-exec-1"));
    expect(defaultProps.onPhaseSelect).toHaveBeenCalledWith("phase-exec-1");
    expect(defaultProps.onPhaseSelect).toHaveBeenCalledTimes(1);
  });

  it("clicking the selected phase calls onPhaseSelect(null) to deselect", () => {
    render(<AuditTrailSidebar {...defaultProps} selectedPhaseId="phase-exec-1" />);
    fireEvent.click(screen.getByTestId("phase-button-phase-exec-1"));
    expect(defaultProps.onPhaseSelect).toHaveBeenCalledWith(null);
  });

  it("View All button calls onPhaseSelect(null)", () => {
    render(<AuditTrailSidebar {...defaultProps} selectedPhaseId="phase-exec-1" />);
    fireEvent.click(screen.getByTestId("view-all-button"));
    expect(defaultProps.onPhaseSelect).toHaveBeenCalledWith(null);
  });

  it("shows summary stats with event count and date range", () => {
    render(<AuditTrailSidebar {...defaultProps} />);
    expect(screen.getByTestId("total-events")).toHaveTextContent("47");
    expect(screen.getByTestId("date-range")).toHaveTextContent("Feb 23, 2026");
  });

  it("shows loading skeleton when isLoading is true", () => {
    render(<AuditTrailSidebar {...defaultProps} isLoading={true} />);
    expect(screen.getByTestId("sidebar-loading")).toBeInTheDocument();
    expect(screen.queryByTestId("audit-trail-sidebar")).not.toBeInTheDocument();
  });

  it("shows duration for each phase", () => {
    render(<AuditTrailSidebar {...defaultProps} />);
    expect(screen.getByTestId("phase-duration-phase-exec-1")).toBeInTheDocument();
    expect(screen.getByTestId("phase-duration-phase-review-1")).toBeInTheDocument();
    expect(screen.getByTestId("phase-duration-phase-merge-1")).toBeInTheDocument();
    // 1m duration for exec, 46s for review, 12s for merge
    expect(screen.getByTestId("phase-duration-phase-exec-1")).toHaveTextContent("1m");
    expect(screen.getByTestId("phase-duration-phase-review-1")).toHaveTextContent("46s");
    expect(screen.getByTestId("phase-duration-phase-merge-1")).toHaveTextContent("12s");
  });

  it("highlights selected phase with aria-pressed and deselects others", () => {
    render(<AuditTrailSidebar {...defaultProps} selectedPhaseId="phase-exec-1" />);
    expect(screen.getByTestId("phase-button-phase-exec-1")).toHaveAttribute(
      "aria-pressed",
      "true"
    );
    expect(screen.getByTestId("phase-button-phase-review-1")).toHaveAttribute(
      "aria-pressed",
      "false"
    );
    expect(screen.getByTestId("phase-button-phase-merge-1")).toHaveAttribute(
      "aria-pressed",
      "false"
    );
  });

  it("renders vertical connector lines between phases but not on the last phase", () => {
    render(<AuditTrailSidebar {...defaultProps} />);
    expect(screen.getByTestId("phase-connector-phase-exec-1")).toBeInTheDocument();
    expect(screen.getByTestId("phase-connector-phase-review-1")).toBeInTheDocument();
    expect(
      screen.queryByTestId("phase-connector-phase-merge-1")
    ).not.toBeInTheDocument();
  });
});
