import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { formatDate, formatRelativeTime, formatDuration } from "./formatters";

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
