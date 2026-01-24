/**
 * Formatting utilities for dates, times, and durations
 *
 * Provides consistent formatting for the application UI.
 */

/** Input types accepted by date formatting functions */
type DateInput = string | number | Date;

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
  if (input === null || input === undefined) {
    return "-";
  }

  try {
    const date = input instanceof Date ? input : new Date(input);

    if (isNaN(date.getTime())) {
      return "-";
    }

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
  if (input === null || input === undefined) {
    return "-";
  }

  try {
    const date = input instanceof Date ? input : new Date(input);

    if (isNaN(date.getTime())) {
      return "-";
    }

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
