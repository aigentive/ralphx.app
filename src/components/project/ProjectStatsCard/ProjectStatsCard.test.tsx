/**
 * ProjectStatsCard component tests
 *
 * Tests loading, error, and success states for the project stats card,
 * including progressive unlock thresholds, collapsible estimates, and
 * Copy as Markdown output.
 *
 * Mocks useProjectStats hook — does not call Tauri invoke directly.
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { act } from "react";
import { render, screen, fireEvent } from "@testing-library/react";
import { ProjectStatsCard } from "./ProjectStatsCard";
import { useProjectStats } from "@/hooks/useProjectStats";
import { useMetricsConfig, useSaveMetricsConfig } from "@/hooks/useMetricsConfig";
import { DEFAULT_METRICS_CONFIG } from "@/types/project-stats";
import type { ProjectStats } from "@/types/project-stats";

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

vi.mock("@/hooks/useProjectStats", () => ({
  useProjectStats: vi.fn(),
}));

vi.mock("@/hooks/useMetricsConfig", () => ({
  useMetricsConfig: vi.fn(),
  useSaveMetricsConfig: vi.fn(),
}));

// Satisfy any transitive import of the api module
vi.mock("@/api/project-stats", () => ({
  projectStatsApi: {
    getProjectStats: vi.fn(),
    getMetricsConfig: vi.fn(),
    saveMetricsConfig: vi.fn(),
  },
}));

const mockedUseProjectStats = vi.mocked(useProjectStats);
const mockedUseMetricsConfig = vi.mocked(useMetricsConfig);
const mockedUseSaveMetricsConfig = vi.mocked(useSaveMetricsConfig);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeStats(overrides: Partial<ProjectStats> = {}): ProjectStats {
  return {
    taskCount: 42,
    tasksCompletedToday: 3,
    tasksCompletedThisWeek: 12,
    tasksCompletedThisMonth: 38,
    agentSuccessRate: 0.85,
    agentSuccessCount: 17,
    agentTotalCount: 20,
    reviewPassRate: 0.9,
    reviewPassCount: 9,
    reviewTotalCount: 10,
    cycleTimeBreakdown: [
      { phase: "Executing", avgMinutes: 60, sampleSize: 10 },
      { phase: "Review", avgMinutes: 30, sampleSize: 10 },
    ],
    eme: null,
    ...overrides,
  };
}

function mockLoading() {
  mockedUseProjectStats.mockReturnValue({
    data: undefined,
    isLoading: true,
    isError: false,
  } as ReturnType<typeof useProjectStats>);
}

function mockError() {
  mockedUseProjectStats.mockReturnValue({
    data: undefined,
    isLoading: false,
    isError: true,
  } as ReturnType<typeof useProjectStats>);
}

function mockSuccess(stats: ProjectStats) {
  mockedUseProjectStats.mockReturnValue({
    data: stats,
    isLoading: false,
    isError: false,
  } as ReturnType<typeof useProjectStats>);
}

function mockDefaultConfig() {
  mockedUseMetricsConfig.mockReturnValue({
    data: DEFAULT_METRICS_CONFIG,
    isLoading: false,
    isError: false,
  } as ReturnType<typeof useMetricsConfig>);

  const mockMutate = vi.fn();
  mockedUseSaveMetricsConfig.mockReturnValue({
    mutate: mockMutate,
    isPending: false,
  } as unknown as ReturnType<typeof useSaveMetricsConfig>);

  return mockMutate;
}

function mockCustomConfig(
  config = {
    simpleBaseHours: 4,
    mediumBaseHours: 8,
    complexBaseHours: 16,
    calendarFactor: 2.0,
  }
) {
  mockedUseMetricsConfig.mockReturnValue({
    data: config,
    isLoading: false,
    isError: false,
  } as ReturnType<typeof useMetricsConfig>);

  const mockMutate = vi.fn();
  mockedUseSaveMetricsConfig.mockReturnValue({
    mutate: mockMutate,
    isPending: false,
  } as unknown as ReturnType<typeof useSaveMetricsConfig>);

  return mockMutate;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ProjectStatsCard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock clipboard API
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  // -------------------------------------------------------------------------
  // Loading state
  // -------------------------------------------------------------------------

  describe("loading state", () => {
    it("renders loading skeleton when isLoading is true", () => {
      mockLoading();
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("project-stats-loading")).toBeInTheDocument();
    });

    it("does not render the card when loading", () => {
      mockLoading();
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByTestId("project-stats-card")).not.toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Error state
  // -------------------------------------------------------------------------

  describe("error state", () => {
    it("renders error message when isError is true", () => {
      mockError();
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("project-stats-error")).toBeInTheDocument();
      expect(screen.getByText("Could not load project stats.")).toBeInTheDocument();
    });

    it("renders error state when data is undefined and not loading", () => {
      mockedUseProjectStats.mockReturnValue({
        data: undefined,
        isLoading: false,
        isError: false,
      } as ReturnType<typeof useProjectStats>);

      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("project-stats-error")).toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Progressive unlock thresholds
  // -------------------------------------------------------------------------

  describe("progressive unlock thresholds", () => {
    it("shows empty state message when taskCount is 0", () => {
      mockSuccess(makeStats({ taskCount: 0 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("project-stats-empty")).toBeInTheDocument();
      expect(
        screen.getByText("Complete tasks to see metrics.")
      ).toBeInTheDocument();
    });

    it("still renders card container with header when taskCount is 0", () => {
      mockSuccess(makeStats({ taskCount: 0 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("project-stats-card")).toBeInTheDocument();
      expect(screen.getByText("Project Stats")).toBeInTheDocument();
    });

    it("shows task count metrics when taskCount >= 1", () => {
      mockSuccess(makeStats({ taskCount: 1, tasksCompletedThisWeek: 0, tasksCompletedToday: 0 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("stat-value-total")).toBeInTheDocument();
      expect(screen.queryByTestId("project-stats-empty")).not.toBeInTheDocument();
    });

    it("hides quality and cycle time sections when taskCount < 5", () => {
      mockSuccess(
        makeStats({
          taskCount: 3,
          agentTotalCount: 10,
          agentSuccessCount: 8,
          agentSuccessRate: 0.8,
          cycleTimeBreakdown: [{ phase: "Executing", avgMinutes: 60, sampleSize: 3 }],
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByText("Quality")).not.toBeInTheDocument();
      expect(screen.queryByText("Cycle Time")).not.toBeInTheDocument();
    });

    it("shows quality and cycle time sections when taskCount >= 5", () => {
      mockSuccess(
        makeStats({
          taskCount: 5,
          agentTotalCount: 5,
          agentSuccessCount: 4,
          agentSuccessRate: 0.8,
          cycleTimeBreakdown: [{ phase: "Executing", avgMinutes: 60, sampleSize: 5 }],
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("Quality")).toBeInTheDocument();
      expect(screen.getByText("Cycle Time")).toBeInTheDocument();
    });

    it("does not show estimates toggle when taskCount < 5", () => {
      mockSuccess(
        makeStats({
          taskCount: 3,
          eme: { lowHours: 8, highHours: 16, taskCount: 3 },
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByTestId("estimates-section")).not.toBeInTheDocument();
    });

    it("does not show estimates section when eme is null (even with taskCount >= 5)", () => {
      mockSuccess(makeStats({ taskCount: 10, eme: null }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByTestId("estimates-section")).not.toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Success state — card renders
  // -------------------------------------------------------------------------

  describe("success state", () => {
    it("renders the stats card container", () => {
      mockSuccess(makeStats());
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("project-stats-card")).toBeInTheDocument();
    });

    it("displays the 'Project Stats' heading", () => {
      mockSuccess(makeStats());
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("Project Stats")).toBeInTheDocument();
    });

    it("passes projectId to the hook", () => {
      mockSuccess(makeStats());
      render(<ProjectStatsCard projectId="proj-abc" />);
      expect(mockedUseProjectStats).toHaveBeenCalledWith("proj-abc");
    });

    it("applies custom className to the outer container", () => {
      mockSuccess(makeStats());
      render(
        <ProjectStatsCard projectId="proj-1" className="custom-class" />
      );
      const card = screen.getByTestId("project-stats-card");
      expect(card).toHaveClass("custom-class");
    });
  });

  // -------------------------------------------------------------------------
  // Task counts
  // -------------------------------------------------------------------------

  describe("task count stats", () => {
    it("renders total task count", () => {
      mockSuccess(makeStats({ taskCount: 55 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("stat-value-total")).toHaveTextContent("55");
    });

    it("renders tasks completed this week", () => {
      mockSuccess(makeStats({ tasksCompletedThisWeek: 7 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("stat-value-this-week")).toHaveTextContent("7");
    });

    it("renders tasks completed today", () => {
      mockSuccess(makeStats({ tasksCompletedToday: 2 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("stat-value-today")).toHaveTextContent("2");
    });
  });

  // -------------------------------------------------------------------------
  // Rate bars (shown only at ≥5 tasks)
  // -------------------------------------------------------------------------

  describe("agent success rate", () => {
    it("renders agent success rate bar when agentTotalCount > 0 and taskCount >= 5", () => {
      mockSuccess(makeStats({ agentTotalCount: 10, agentSuccessCount: 8, agentSuccessRate: 0.8 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("agent-success-rate")).toBeInTheDocument();
    });

    it("shows agent success percentage", () => {
      mockSuccess(makeStats({ agentTotalCount: 10, agentSuccessCount: 8, agentSuccessRate: 0.8 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("80%")).toBeInTheDocument();
    });

    it("shows agent pass/total counts", () => {
      mockSuccess(makeStats({ agentTotalCount: 20, agentSuccessCount: 17, agentSuccessRate: 0.85 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("(17/20)")).toBeInTheDocument();
    });

    it("hides agent success rate when agentTotalCount is 0", () => {
      mockSuccess(makeStats({ agentTotalCount: 0, agentSuccessCount: 0, agentSuccessRate: 0 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByTestId("agent-success-rate")).not.toBeInTheDocument();
    });

    it("renders progress bar with correct aria attributes", () => {
      mockSuccess(makeStats({ agentTotalCount: 10, agentSuccessCount: 8, agentSuccessRate: 0.8 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      const bar = screen.getByRole("progressbar", { name: /agent success/i });
      expect(bar).toHaveAttribute("aria-valuenow", "80");
      expect(bar).toHaveAttribute("aria-valuemin", "0");
      expect(bar).toHaveAttribute("aria-valuemax", "100");
    });
  });

  describe("review pass rate", () => {
    it("renders review pass rate bar when reviewTotalCount > 0 and taskCount >= 5", () => {
      mockSuccess(makeStats({ reviewTotalCount: 10, reviewPassCount: 9, reviewPassRate: 0.9 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("review-pass-rate")).toBeInTheDocument();
    });

    it("shows review pass percentage", () => {
      mockSuccess(makeStats({ reviewTotalCount: 10, reviewPassCount: 9, reviewPassRate: 0.9 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("90%")).toBeInTheDocument();
    });

    it("hides review pass rate when reviewTotalCount is 0", () => {
      mockSuccess(makeStats({ reviewTotalCount: 0, reviewPassCount: 0, reviewPassRate: 0 }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByTestId("review-pass-rate")).not.toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Cycle time breakdown
  // -------------------------------------------------------------------------

  describe("cycle time breakdown", () => {
    it("renders cycle time phases when breakdown is non-empty and taskCount >= 5", () => {
      mockSuccess(
        makeStats({
          cycleTimeBreakdown: [
            { phase: "Executing", avgMinutes: 60, sampleSize: 10 },
            { phase: "Review", avgMinutes: 30, sampleSize: 10 },
          ],
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("cycle-phase-Executing")).toBeInTheDocument();
      expect(screen.getByTestId("cycle-phase-Review")).toBeInTheDocument();
    });

    it("renders phase labels in the cycle time section", () => {
      mockSuccess(
        makeStats({
          cycleTimeBreakdown: [
            { phase: "Executing", avgMinutes: 120, sampleSize: 5 },
          ],
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("Executing")).toBeInTheDocument();
    });

    it("displays duration in hours", () => {
      mockSuccess(
        makeStats({
          cycleTimeBreakdown: [
            { phase: "Executing", avgMinutes: 60, sampleSize: 5 }, // 1.0h
          ],
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("1.0h")).toBeInTheDocument();
    });

    it("does not render cycle time section when breakdown is empty", () => {
      mockSuccess(makeStats({ cycleTimeBreakdown: [] }));
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByText("Cycle Time")).not.toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Collapsible Estimates section
  // -------------------------------------------------------------------------

  describe("collapsible estimates section", () => {
    const statsWithEme = makeStats({
      taskCount: 10,
      eme: { lowHours: 8, highHours: 16, taskCount: 10 },
    });

    it("shows estimates toggle when taskCount >= 5 and eme is present", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("estimates-section")).toBeInTheDocument();
      expect(screen.getByTestId("estimates-toggle")).toBeInTheDocument();
    });

    it("estimates section is collapsed by default (content hidden)", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByTestId("eme-value")).not.toBeInTheDocument();
      expect(
        screen.queryByText("Based on task complexity analysis.")
      ).not.toBeInTheDocument();
    });

    it("expands estimates section when toggle is clicked", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      fireEvent.click(screen.getByTestId("estimates-toggle"));
      expect(screen.getByTestId("eme-value")).toBeInTheDocument();
      expect(screen.getByText(/~8–16h/)).toBeInTheDocument();
    });

    it("shows methodology note when expanded", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      fireEvent.click(screen.getByTestId("estimates-toggle"));
      expect(
        screen.getByText(
          "Based on task complexity analysis. Ranges are conservative estimates."
        )
      ).toBeInTheDocument();
    });

    it("hides formula content by default after expanding estimates", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      fireEvent.click(screen.getByTestId("estimates-toggle"));
      expect(screen.queryByTestId("formula-content")).not.toBeInTheDocument();
    });

    it("shows formula content when formula toggle is clicked", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      fireEvent.click(screen.getByTestId("estimates-toggle"));
      fireEvent.click(screen.getByTestId("formula-toggle"));
      expect(screen.getByTestId("formula-content")).toBeInTheDocument();
      expect(screen.getByText(/Simple.*= 2h/)).toBeInTheDocument();
      expect(screen.getByText(/Medium.*= 4h/)).toBeInTheDocument();
      expect(screen.getByText(/Complex.*= 8h/)).toBeInTheDocument();
      expect(screen.getByText(/×1\.5 calendar factor/)).toBeInTheDocument();
    });

    it("collapses estimates section when toggle is clicked again", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      fireEvent.click(screen.getByTestId("estimates-toggle"));
      expect(screen.getByTestId("eme-value")).toBeInTheDocument();
      fireEvent.click(screen.getByTestId("estimates-toggle"));
      expect(screen.queryByTestId("eme-value")).not.toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Conditional section rendering
  // -------------------------------------------------------------------------

  describe("conditional section rendering", () => {
    it("does not render quality section when both totals are 0", () => {
      mockSuccess(
        makeStats({
          agentTotalCount: 0,
          reviewTotalCount: 0,
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.queryByText("Quality")).not.toBeInTheDocument();
    });

    it("renders quality section when at least one rate has data", () => {
      mockSuccess(
        makeStats({
          agentTotalCount: 5,
          agentSuccessCount: 4,
          agentSuccessRate: 0.8,
          reviewTotalCount: 0,
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByText("Quality")).toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Copy as Markdown
  // -------------------------------------------------------------------------

  describe("Copy as Markdown", () => {
    it("renders Copy as Markdown button", () => {
      mockSuccess(makeStats());
      render(<ProjectStatsCard projectId="proj-1" />);
      expect(screen.getByTestId("copy-markdown-button")).toBeInTheDocument();
      expect(screen.getByText("Copy as Markdown")).toBeInTheDocument();
    });

    it("calls clipboard.writeText when button is clicked", async () => {
      mockSuccess(makeStats());
      render(<ProjectStatsCard projectId="proj-1" />);
      await act(async () => {
        fireEvent.click(screen.getByTestId("copy-markdown-button"));
      });
      expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(1);
    });

    it("markdown output contains quality section with correct rates", async () => {
      mockSuccess(
        makeStats({
          agentSuccessRate: 0.85,
          agentSuccessCount: 17,
          agentTotalCount: 20,
          reviewPassRate: 0.9,
          reviewPassCount: 9,
          reviewTotalCount: 10,
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      await act(async () => {
        fireEvent.click(screen.getByTestId("copy-markdown-button"));
      });

      const written = vi.mocked(navigator.clipboard.writeText).mock.calls[0]?.[0] ?? "";
      expect(written).toContain("## Project Stats");
      expect(written).toContain("### Quality");
      expect(written).toContain("85%");
      expect(written).toContain("17/20");
      expect(written).toContain("90%");
    });

    it("markdown output contains throughput section with task counts", async () => {
      mockSuccess(
        makeStats({
          tasksCompletedThisWeek: 12,
          tasksCompletedThisMonth: 38,
        })
      );
      render(<ProjectStatsCard projectId="proj-1" />);
      await act(async () => {
        fireEvent.click(screen.getByTestId("copy-markdown-button"));
      });

      const written = vi.mocked(navigator.clipboard.writeText).mock.calls[0]?.[0] ?? "";
      expect(written).toContain("### Throughput");
      expect(written).toContain("12 this week");
      expect(written).toContain("38 this month");
    });

    it("markdown output includes EME section when eme is present", async () => {
      mockSuccess(
        makeStats({
          taskCount: 10,
          eme: { lowHours: 8, highHours: 16, taskCount: 10 },
        })
      );
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);
      await act(async () => {
        fireEvent.click(screen.getByTestId("copy-markdown-button"));
      });

      const written = vi.mocked(navigator.clipboard.writeText).mock.calls[0]?.[0] ?? "";
      expect(written).toContain("### Estimated Manual Effort");
      expect(written).toContain("~8–16 hours");
    });

    it("markdown output omits EME section when eme is null", async () => {
      mockSuccess(makeStats({ eme: null }));
      render(<ProjectStatsCard projectId="proj-1" />);
      await act(async () => {
        fireEvent.click(screen.getByTestId("copy-markdown-button"));
      });

      const written = vi.mocked(navigator.clipboard.writeText).mock.calls[0]?.[0] ?? "";
      expect(written).not.toContain("### Estimated Manual Effort");
    });

    it("markdown output contains RalphX attribution footer", async () => {
      mockSuccess(makeStats());
      render(<ProjectStatsCard projectId="proj-1" />);
      await act(async () => {
        fireEvent.click(screen.getByTestId("copy-markdown-button"));
      });

      const written = vi.mocked(navigator.clipboard.writeText).mock.calls[0]?.[0] ?? "";
      expect(written).toContain("your data never leaves your machine");
    });
  });

  // -------------------------------------------------------------------------
  // Calibration UI
  // -------------------------------------------------------------------------

  describe("calibration UI", () => {
    const statsWithEme = makeStats({
      taskCount: 10,
      eme: { lowHours: 8, highHours: 16, taskCount: 10 },
    });

    it("shows calibration section when formula is expanded", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);

      fireEvent.click(screen.getByTestId("estimates-toggle"));
      fireEvent.click(screen.getByTestId("formula-toggle"));

      expect(screen.getByTestId("calibration-section")).toBeInTheDocument();
    });

    it("renders all 4 calibration inputs with default values", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);

      fireEvent.click(screen.getByTestId("estimates-toggle"));
      fireEvent.click(screen.getByTestId("formula-toggle"));

      expect(screen.getByTestId("calibrate-simpleBaseHours")).toHaveValue(2);
      expect(screen.getByTestId("calibrate-mediumBaseHours")).toHaveValue(4);
      expect(screen.getByTestId("calibrate-complexBaseHours")).toHaveValue(8);
      expect(screen.getByTestId("calibrate-calendarFactor")).toHaveValue(1.5);
    });

    it("does not show calibrated badge when using default config", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);

      fireEvent.click(screen.getByTestId("estimates-toggle"));

      expect(screen.queryByText("calibrated")).not.toBeInTheDocument();
    });

    it("shows calibrated badge when config differs from defaults", () => {
      mockSuccess(statsWithEme);
      mockCustomConfig();
      render(<ProjectStatsCard projectId="proj-1" />);

      fireEvent.click(screen.getByTestId("estimates-toggle"));

      expect(screen.getByText("calibrated")).toBeInTheDocument();
    });

    it("does not show reset button when using default config", () => {
      mockSuccess(statsWithEme);
      mockDefaultConfig();
      render(<ProjectStatsCard projectId="proj-1" />);

      fireEvent.click(screen.getByTestId("estimates-toggle"));
      fireEvent.click(screen.getByTestId("formula-toggle"));

      expect(screen.queryByTestId("calibration-reset")).not.toBeInTheDocument();
    });

    it("shows reset button when using custom config", () => {
      mockSuccess(statsWithEme);
      mockCustomConfig();
      render(<ProjectStatsCard projectId="proj-1" />);

      fireEvent.click(screen.getByTestId("estimates-toggle"));
      fireEvent.click(screen.getByTestId("formula-toggle"));

      expect(screen.getByTestId("calibration-reset")).toBeInTheDocument();
    });

    it("calls saveConfig with default values when reset is clicked", () => {
      mockSuccess(statsWithEme);
      const mockMutate = mockCustomConfig();
      render(<ProjectStatsCard projectId="proj-1" />);

      fireEvent.click(screen.getByTestId("estimates-toggle"));
      fireEvent.click(screen.getByTestId("formula-toggle"));
      fireEvent.click(screen.getByTestId("calibration-reset"));

      expect(mockMutate).toHaveBeenCalledWith(DEFAULT_METRICS_CONFIG);
    });
  });
});
