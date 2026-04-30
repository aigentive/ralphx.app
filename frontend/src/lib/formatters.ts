/**
 * Formatting utilities for dates, times, and durations
 *
 * Provides consistent formatting for the application UI.
 */

/** Input types accepted by date formatting functions */
type DateInput = string | number | Date | null | undefined;

function toValidDate(input: DateInput): Date | null {
  if (input === null || input === undefined) {
    return null;
  }

  const date = input instanceof Date ? input : new Date(input);
  return Number.isNaN(date.getTime()) ? null : date;
}

/**
 * Format a date for display
 *
 * @param input - ISO string, timestamp, or Date object
 * @returns Formatted date string or "-" if invalid
 *
 * @example
 * ```ts
 * formatDate("2026-01-24T12:00:00Z") // "Jan 24, 2026"
 * formatDate(new Date()) // "Jan 24, 2026"
 * formatDate(null) // "-"
 * ```
 */
export function formatDate(input: DateInput): string {
  try {
    const date = toValidDate(input);
    if (!date) return "-";

    return date.toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return "-";
  }
}

/**
 * Format a date as relative time from now
 *
 * @param input - ISO string, timestamp, or Date object
 * @returns Relative time string (e.g., "2 hours ago") or "-" if invalid
 *
 * @example
 * ```ts
 * formatRelativeTime("2026-01-24T10:00:00Z") // "2 hours ago"
 * formatRelativeTime("2026-01-22T12:00:00Z") // "2 days ago"
 * ```
 */
export function formatRelativeTime(input: DateInput): string {
  try {
    const date = toValidDate(input);
    if (!date) return "-";

    const now = Date.now();
    const diff = now - date.getTime();
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    const weeks = Math.floor(days / 7);
    const months = Math.floor(days / 30);
    const years = Math.floor(days / 365);

    if (seconds < 5) {
      return "just now";
    }
    if (seconds < 60) {
      return `${seconds} seconds ago`;
    }
    if (minutes < 60) {
      return minutes === 1 ? "1 minute ago" : `${minutes} minutes ago`;
    }
    if (hours < 24) {
      return hours === 1 ? "1 hour ago" : `${hours} hours ago`;
    }
    if (days < 7) {
      return days === 1 ? "1 day ago" : `${days} days ago`;
    }
    if (weeks < 4) {
      return weeks === 1 ? "1 week ago" : `${weeks} weeks ago`;
    }
    if (months < 12) {
      return months === 1 ? "1 month ago" : `${months} months ago`;
    }
    return years === 1 ? "1 year ago" : `${years} years ago`;
  } catch {
    return "-";
  }
}

/**
 * Format a duration in seconds to human-readable format
 *
 * @param seconds - Duration in seconds
 * @returns Formatted duration string (e.g., "5m 30s") or "-" if invalid
 *
 * @example
 * ```ts
 * formatDuration(90) // "1m 30s"
 * formatDuration(3661) // "1h 1m 1s"
 * formatDuration(30) // "30s"
 * ```
 */
export function formatDuration(seconds: number): string {
  if (seconds === null || seconds === undefined || isNaN(seconds)) {
    return "-";
  }

  if (seconds < 0) {
    return "-";
  }

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  const parts: string[] = [];

  if (hours > 0) {
    parts.push(`${hours}h`);
  }
  if (minutes > 0 || hours > 0) {
    parts.push(`${minutes}m`);
  }
  if (secs > 0 || parts.length === 0) {
    parts.push(`${secs}s`);
  }

  return parts.join(" ");
}

/**
 * Format elapsed seconds as a compact duration for live timers.
 *
 * @param seconds - Elapsed seconds, or null if the timer has not started
 * @returns Formatted string like "45s" or "3m 12s", or "—" if null
 *
 * @example
 * ```ts
 * formatElapsedTime(45)   // "45s"
 * formatElapsedTime(192)  // "3m 12s"
 * formatElapsedTime(null) // "—"
 * ```
 */
export function formatElapsedTime(seconds: number | null): string {
  if (seconds === null) return "\u2014";
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (mins === 0) return `${secs}s`;
  return `${mins}m ${secs}s`;
}

/**
 * Format minutes into human-readable time: "18m", "7h 30m", "2h".
 * Rules: < 60 min → "Xm", >= 60 min → "Xh Ym", if Y is 0 → "Xh"
 */
export function formatMinutesHuman(minutes: number): string {
  if (!minutes || minutes <= 0) return "0m";
  if (minutes < 60) return `${Math.round(minutes)}m`;
  const h = Math.floor(minutes / 60);
  const m = Math.round(minutes % 60);
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

/**
 * Format a date/time for version history display.
 * Format: "MMM D, h:mm AM/PM" — includes year if the date is not in the current year.
 *
 * @param input - ISO string, timestamp, or Date object
 * @returns Formatted string (e.g., "Mar 18, 11:30 AM") or "-" if invalid
 *
 * @example
 * ```ts
 * formatDateTime("2026-03-18T11:30:00Z") // "Mar 18, 11:30 AM"
 * formatDateTime("2025-12-01T09:00:00Z") // "Dec 1, 2025, 9:00 AM"
 * ```
 */
export function formatDateTime(input: DateInput): string {
  try {
    const date = toValidDate(input);
    if (!date) return "-";

    const currentYear = new Date().getFullYear();
    const dateYear = date.getFullYear();

    const options: Intl.DateTimeFormatOptions = {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    };

    if (dateYear !== currentYear) {
      options.year = "numeric";
    }

    return date.toLocaleString("en-US", options);
  } catch {
    return "-";
  }
}

export interface HumanTimestamp {
  label: string;
  title: string;
}

const HUMAN_TIMESTAMP_ABSOLUTE_AFTER_MS = 7 * 24 * 60 * 60 * 1000;

function formatAbsoluteTimestampLabel(date: Date): string {
  const timeLabel = date.toLocaleTimeString("en-US", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
  const dateOptions: Intl.DateTimeFormatOptions = {
    month: "short",
    day: "numeric",
  };

  if (date.getFullYear() !== new Date().getFullYear()) {
    dateOptions.year = "numeric";
  }

  return `${timeLabel} * ${date.toLocaleDateString("en-US", dateOptions)}`;
}

function formatTimestampTitle(date: Date): string {
  return date.toLocaleString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
  });
}

export function formatHumanTimestamp(input: DateInput): HumanTimestamp {
  try {
    const date = toValidDate(input);
    if (!date) return { label: "-", title: "" };

    const diffMs = Math.max(0, Date.now() - date.getTime());
    const title = formatTimestampTitle(date);

    if (diffMs >= HUMAN_TIMESTAMP_ABSOLUTE_AFTER_MS) {
      return {
        label: formatAbsoluteTimestampLabel(date),
        title,
      };
    }

    const minutes = Math.floor(diffMs / 60_000);
    if (minutes < 1) {
      return { label: "just now", title };
    }

    if (minutes < 60) {
      return {
        label: minutes === 1 ? "1 minute ago" : `${minutes} minutes ago`,
        title,
      };
    }

    const hours = Math.floor(minutes / 60);
    if (hours < 24) {
      return {
        label: hours === 1 ? "1 hour ago" : `${hours} hours ago`,
        title,
      };
    }

    const days = Math.floor(hours / 24);
    return {
      label: days === 1 ? "1 day ago" : `${days} days ago`,
      title,
    };
  } catch {
    return { label: "-", title: "" };
  }
}

export function formatHumanTimestampLabel(input: DateInput): string {
  return formatHumanTimestamp(input).label;
}

export function formatHumanTimestampTitle(input: DateInput): string {
  return formatHumanTimestamp(input).title;
}
