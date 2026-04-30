import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  formatDate,
  formatRelativeTime,
  formatDuration,
  formatDateTime,
  formatHumanTimestamp,
} from "./formatters";

describe("formatDate", () => {
  it("should format ISO date string", () => {
    const result = formatDate("2026-01-24T12:00:00Z");
    expect(result).toBeDefined();
    expect(typeof result).toBe("string");
  });

  it("should format Date object", () => {
    const date = new Date("2026-01-24T12:00:00Z");
    const result = formatDate(date);
    expect(result).toBeDefined();
    expect(typeof result).toBe("string");
  });

  it("should format timestamp number", () => {
    const timestamp = new Date("2026-01-24T12:00:00Z").getTime();
    const result = formatDate(timestamp);
    expect(result).toBeDefined();
    expect(typeof result).toBe("string");
  });

  it("should include date components", () => {
    const result = formatDate("2026-01-24T12:00:00Z");
    // Should include month, day, year in some format
    expect(result).toMatch(/\d/);
  });

  it("should handle null gracefully", () => {
    const result = formatDate(null as unknown as string);
    expect(result).toBe("-");
  });

  it("should handle undefined gracefully", () => {
    const result = formatDate(undefined as unknown as string);
    expect(result).toBe("-");
  });

  it("should handle invalid date string gracefully", () => {
    const result = formatDate("not-a-date");
    expect(result).toBe("-");
  });
});

describe("formatRelativeTime", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Set current time to 2026-01-24 12:00:00 UTC
    vi.setSystemTime(new Date("2026-01-24T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("should format time just now", () => {
    const result = formatRelativeTime("2026-01-24T12:00:00Z");
    expect(result).toMatch(/just now|now|0 seconds|seconds ago/i);
  });

  it("should format seconds ago", () => {
    const result = formatRelativeTime("2026-01-24T11:59:30Z");
    expect(result).toMatch(/seconds?|just now/i);
  });

  it("should format minutes ago", () => {
    const result = formatRelativeTime("2026-01-24T11:55:00Z");
    expect(result).toMatch(/5.*min|minutes?/i);
  });

  it("should format hours ago", () => {
    const result = formatRelativeTime("2026-01-24T10:00:00Z");
    expect(result).toMatch(/2.*hour|hours?/i);
  });

  it("should format days ago", () => {
    const result = formatRelativeTime("2026-01-22T12:00:00Z");
    expect(result).toMatch(/2.*day|days?/i);
  });

  it("should format weeks ago", () => {
    const result = formatRelativeTime("2026-01-10T12:00:00Z");
    expect(result).toMatch(/2.*week|weeks?/i);
  });

  it("should handle Date object", () => {
    const date = new Date("2026-01-24T11:00:00Z");
    const result = formatRelativeTime(date);
    expect(result).toMatch(/hour/i);
  });

  it("should handle timestamp number", () => {
    const timestamp = new Date("2026-01-24T11:00:00Z").getTime();
    const result = formatRelativeTime(timestamp);
    expect(result).toMatch(/hour/i);
  });

  it("should handle null gracefully", () => {
    const result = formatRelativeTime(null as unknown as string);
    expect(result).toBe("-");
  });

  it("should handle undefined gracefully", () => {
    const result = formatRelativeTime(undefined as unknown as string);
    expect(result).toBe("-");
  });
});

describe("formatDuration", () => {
  it("should format seconds only", () => {
    const result = formatDuration(30);
    expect(result).toMatch(/30\s*s/i);
  });

  it("should format minutes and seconds", () => {
    const result = formatDuration(90);
    expect(result).toMatch(/1\s*m.*30\s*s/i);
  });

  it("should format hours, minutes, and seconds", () => {
    const result = formatDuration(3661);
    expect(result).toMatch(/1\s*h.*1\s*m/i);
  });

  it("should format zero duration", () => {
    const result = formatDuration(0);
    expect(result).toMatch(/0\s*s/i);
  });

  it("should format large durations", () => {
    const result = formatDuration(86400); // 24 hours
    expect(result).toMatch(/24\s*h|1\s*d/i);
  });

  it("should handle negative values gracefully", () => {
    const result = formatDuration(-10);
    expect(result).toBe("-");
  });

  it("should handle null gracefully", () => {
    const result = formatDuration(null as unknown as number);
    expect(result).toBe("-");
  });

  it("should handle undefined gracefully", () => {
    const result = formatDuration(undefined as unknown as number);
    expect(result).toBe("-");
  });

  it("should handle NaN gracefully", () => {
    const result = formatDuration(NaN);
    expect(result).toBe("-");
  });

  it("should format exactly 1 minute", () => {
    const result = formatDuration(60);
    expect(result).toMatch(/1\s*m/i);
  });

  it("should format exactly 1 hour", () => {
    const result = formatDuration(3600);
    expect(result).toMatch(/1\s*h/i);
  });
});

describe("formatDateTime", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Set current time to 2026-03-19 (same year for most tests)
    vi.setSystemTime(new Date("2026-03-19T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns '-' for null", () => {
    expect(formatDateTime(null as unknown as string)).toBe("-");
  });

  it("returns '-' for undefined", () => {
    expect(formatDateTime(undefined as unknown as string)).toBe("-");
  });

  it("returns '-' for invalid date string", () => {
    expect(formatDateTime("not-a-date")).toBe("-");
  });

  it("formats same-year date without year component", () => {
    const result = formatDateTime("2026-03-18T11:30:00Z");
    // Should include month and time but no year
    expect(result).toMatch(/Mar/i);
    expect(result).toMatch(/18/);
    expect(result).not.toMatch(/2026/);
  });

  it("formats cross-year date with year component", () => {
    const result = formatDateTime("2025-12-01T09:00:00Z");
    // Should include year for dates in a different year
    expect(result).toMatch(/Dec/i);
    expect(result).toMatch(/2025/);
  });

  it("formats time with AM/PM", () => {
    const result = formatDateTime("2026-01-15T11:30:00Z");
    expect(result).toMatch(/AM|PM/i);
  });

  it("includes minutes in the output", () => {
    const result = formatDateTime("2026-02-10T14:45:00Z");
    expect(result).toMatch(/\d{1,2}:\d{2}/);
  });

  it("accepts a Date object", () => {
    const date = new Date("2026-03-01T10:00:00Z");
    const result = formatDateTime(date);
    expect(result).not.toBe("-");
    expect(typeof result).toBe("string");
  });

  it("accepts a timestamp number", () => {
    const ts = new Date("2026-03-05T08:00:00Z").getTime();
    const result = formatDateTime(ts);
    expect(result).not.toBe("-");
    expect(typeof result).toBe("string");
  });
});

describe("formatHumanTimestamp", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 3, 25, 16, 33, 0));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("formats sub-minute timestamps as just now", () => {
    const result = formatHumanTimestamp(new Date(2026, 3, 25, 16, 32, 30));

    expect(result.label).toBe("just now");
    expect(result.title).toMatch(/Apr 25, 2026/);
    expect(result.title).toMatch(/4:32 PM/);
  });

  it("formats singular minutes and hours", () => {
    expect(formatHumanTimestamp(new Date(2026, 3, 25, 16, 32, 0)).label).toBe("1 minute ago");
    expect(formatHumanTimestamp(new Date(2026, 3, 25, 15, 33, 0)).label).toBe("1 hour ago");
  });

  it("formats plural hours and days inside the 7-day window", () => {
    expect(formatHumanTimestamp(new Date(2026, 3, 25, 14, 33, 0)).label).toBe("2 hours ago");
    expect(formatHumanTimestamp(new Date(2026, 3, 23, 16, 33, 0)).label).toBe("2 days ago");
  });

  it("uses the time and date label once the timestamp is outside the 7-day window", () => {
    const result = formatHumanTimestamp(new Date(2026, 3, 17, 16, 33, 0));

    expect(result.label).toBe("4:33 PM * Apr 17");
    expect(result.title).toBe("Apr 17, 2026, 4:33 PM");
  });

  it("includes the year in the absolute label when needed", () => {
    const result = formatHumanTimestamp(new Date(2025, 11, 31, 9, 5, 0));

    expect(result.label).toBe("9:05 AM * Dec 31, 2025");
    expect(result.title).toBe("Dec 31, 2025, 9:05 AM");
  });

  it("handles invalid timestamps", () => {
    expect(formatHumanTimestamp("not-a-date")).toEqual({ label: "-", title: "" });
  });
});
