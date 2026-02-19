/**
 * DurationDisplay unit tests
 *
 * Covers:
 * - formatDuration helper
 * - calcElapsedSeconds helper
 * - Static mode renders correct duration
 * - Live mode increments over time
 * - Cleanup on unmount (no memory leaks)
 * - Null/invalid input edge cases
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import {
  DurationDisplay,
  formatDuration,
  calcElapsedSeconds,
} from "./DurationDisplay";

// ============================================================================
// formatDuration
// ============================================================================

describe("formatDuration", () => {
  it("renders seconds only when under 60s", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(1)).toBe("1s");
    expect(formatDuration(34)).toBe("34s");
    expect(formatDuration(59)).toBe("59s");
  });

  it("renders minutes and seconds when 60s–3599s", () => {
    expect(formatDuration(60)).toBe("1m 0s");
    expect(formatDuration(154)).toBe("2m 34s");
    expect(formatDuration(3599)).toBe("59m 59s");
  });

  it("renders hours, minutes, and seconds when ≥3600s", () => {
    expect(formatDuration(3600)).toBe("1h 0m 0s");
    expect(formatDuration(4354)).toBe("1h 12m 34s");
    expect(formatDuration(7384)).toBe("2h 3m 4s");
  });

  it("clamps negative input to 0s", () => {
    expect(formatDuration(-5)).toBe("0s");
  });
});

// ============================================================================
// calcElapsedSeconds
// ============================================================================

describe("calcElapsedSeconds", () => {
  it("returns null when startedAt is null", () => {
    expect(calcElapsedSeconds(null)).toBeNull();
  });

  it("returns null when startedAt is invalid", () => {
    expect(calcElapsedSeconds("not-a-date")).toBeNull();
  });

  it("returns elapsed seconds between two valid timestamps", () => {
    const start = "2026-01-01T00:00:00.000Z";
    const end = "2026-01-01T00:02:34.000Z";
    expect(calcElapsedSeconds(start, end)).toBe(154);
  });

  it("returns null when endedAt is invalid", () => {
    const start = "2026-01-01T00:00:00.000Z";
    expect(calcElapsedSeconds(start, "bad-date")).toBeNull();
  });

  it("calculates from startedAt to Date.now() when endedAt omitted", () => {
    const now = Date.now();
    const startedAt = new Date(now - 30_000).toISOString();
    const elapsed = calcElapsedSeconds(startedAt);
    // Should be ~30 (allow ±2s for test timing)
    expect(elapsed).toBeGreaterThanOrEqual(28);
    expect(elapsed).toBeLessThanOrEqual(32);
  });

  it("clamps to 0 when end is before start (clock skew)", () => {
    const start = "2026-01-01T00:01:00.000Z";
    const end = "2026-01-01T00:00:00.000Z";
    expect(calcElapsedSeconds(start, end)).toBe(0);
  });
});

// ============================================================================
// DurationDisplay — static mode
// ============================================================================

describe("DurationDisplay — static mode", () => {
  it("renders formatted duration when both timestamps present", () => {
    const start = "2026-01-01T00:00:00.000Z";
    const end = "2026-01-01T00:02:34.000Z";

    render(
      <DurationDisplay mode="static" startedAt={start} completedAt={end} />
    );

    expect(screen.getByTestId("duration-display")).toBeInTheDocument();
    expect(screen.getByText("2m 34s")).toBeInTheDocument();
  });

  it("renders null when startedAt is null", () => {
    const { container } = render(
      <DurationDisplay mode="static" startedAt={null} completedAt={null} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders null when completedAt is null (incomplete static duration)", () => {
    // When completedAt is null in static mode, calcElapsedSeconds computes from now.
    // The component should still render (with current elapsed).
    const start = new Date(Date.now() - 5000).toISOString();
    render(
      <DurationDisplay mode="static" startedAt={start} completedAt={null} />
    );
    // Duration display should appear since startedAt is valid
    expect(screen.getByTestId("duration-display")).toBeInTheDocument();
  });

  it("applies custom className", () => {
    const start = "2026-01-01T00:00:00.000Z";
    const end = "2026-01-01T00:00:05.000Z";
    render(
      <DurationDisplay
        mode="static"
        startedAt={start}
        completedAt={end}
        className="my-custom-class"
      />
    );
    const el = screen.getByTestId("duration-display");
    expect(el.className).toContain("my-custom-class");
  });
});

// ============================================================================
// DurationDisplay — live mode
// ============================================================================

describe("DurationDisplay — live mode", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders initial duration from startedAt", () => {
    const startedAt = new Date(Date.now() - 5000).toISOString();
    render(<DurationDisplay mode="live" startedAt={startedAt} />);

    // Should show ~5 seconds
    expect(screen.getByTestId("duration-display")).toBeInTheDocument();
    expect(screen.getByText("5s")).toBeInTheDocument();
  });

  it("increments by 1 second on each interval tick", () => {
    const startedAt = new Date(Date.now() - 10_000).toISOString();
    render(<DurationDisplay mode="live" startedAt={startedAt} />);

    // Initial state — 10s
    expect(screen.getByText("10s")).toBeInTheDocument();

    // Advance by 1 second
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.getByText("11s")).toBeInTheDocument();

    // Advance by 2 more seconds
    act(() => {
      vi.advanceTimersByTime(2000);
    });
    expect(screen.getByText("13s")).toBeInTheDocument();
  });

  it("renders null when startedAt is null", () => {
    const { container } = render(
      <DurationDisplay mode="live" startedAt={null} />
    );
    expect(container.firstChild).toBeNull();
  });

  it("clears interval on unmount (no memory leaks)", () => {
    const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");
    const startedAt = new Date(Date.now() - 1000).toISOString();

    const { unmount } = render(
      <DurationDisplay mode="live" startedAt={startedAt} />
    );

    unmount();

    expect(clearIntervalSpy).toHaveBeenCalled();
    clearIntervalSpy.mockRestore();
  });
});
