/**
 * Tests for TaskMetricsCard component
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { TaskMetricsCard } from "./TaskMetricsCard";

// Mock useTaskMetrics so we control the query state
vi.mock("@/hooks/useTaskMetrics", () => ({
  useTaskMetrics: vi.fn(),
}));

import { useTaskMetrics } from "@/hooks/useTaskMetrics";

const mockMetrics = {
  stepCount: 5,
  completedStepCount: 3,
  reviewCount: 2,
  approvedReviewCount: 1,
  executionMinutes: 8,
  totalAgeHours: 2,
};

describe("TaskMetricsCard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows loading spinner when isLoading is true", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    const { container } = render(<TaskMetricsCard taskId="task-1" />);
    // Loader2 renders as an SVG with animate-spin class
    const spinner = container.querySelector(".animate-spin");
    expect(spinner).toBeInTheDocument();
  });

  it("returns null when isError is true", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: true,
    } as ReturnType<typeof useTaskMetrics>);

    const { container } = render(<TaskMetricsCard taskId="task-1" />);
    expect(container.firstChild).toBeNull();
  });

  it("returns null when metrics data is undefined", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: undefined,
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    const { container } = render(<TaskMetricsCard taskId="task-1" />);
    expect(container.firstChild).toBeNull();
  });

  it("shows Simple complexity badge for low execution time and low step count", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, executionMinutes: 5, stepCount: 3 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByTestId("complexity-badge")).toHaveTextContent("Simple");
  });

  it("shows Medium complexity badge for medium execution time", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, executionMinutes: 15, stepCount: 4 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByTestId("complexity-badge")).toHaveTextContent("Medium");
  });

  it("shows Complex complexity badge for high execution time", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, executionMinutes: 45, stepCount: 3 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByTestId("complexity-badge")).toHaveTextContent("Complex");
  });

  it("shows Complex complexity badge for high step count", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, executionMinutes: 5, stepCount: 12 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByTestId("complexity-badge")).toHaveTextContent("Complex");
  });

  it("shows step count", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, stepCount: 5, completedStepCount: 3 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByText("3 / 5 completed")).toBeInTheDocument();
  });

  it("shows No steps when stepCount is 0", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, stepCount: 0, completedStepCount: 0 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByText("No steps")).toBeInTheDocument();
  });

  it("shows review count", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, reviewCount: 2 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByText("2 cycles")).toBeInTheDocument();
  });

  it("shows singular review cycle when reviewCount is 1", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, reviewCount: 1 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByText("1 cycle")).toBeInTheDocument();
  });

  it("shows No reviews when reviewCount is 0", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, reviewCount: 0 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByText("No reviews")).toBeInTheDocument();
  });

  it("shows execution time in minutes", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, executionMinutes: 8 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByText("8 min")).toBeInTheDocument();
  });

  it("shows dash when executionMinutes is 0", () => {
    vi.mocked(useTaskMetrics).mockReturnValue({
      data: { ...mockMetrics, executionMinutes: 0 },
      isLoading: false,
      isError: false,
    } as ReturnType<typeof useTaskMetrics>);

    render(<TaskMetricsCard taskId="task-1" />);
    expect(screen.getByText("—")).toBeInTheDocument();
  });
});
