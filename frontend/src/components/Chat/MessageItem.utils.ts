/**
 * MessageItem.utils - Utility functions for MessageItem
 */

import {
  formatHumanTimestampLabel,
  formatHumanTimestampTitle,
} from "@/lib/formatters";

/**
 * Format a timestamp for display in chat messages
 * Shows relative time for recent messages, absolute time for older ones
 */
export function formatTimestamp(createdAt: string): string {
  return formatHumanTimestampLabel(createdAt);
}

export function formatTimestampTitle(createdAt: string): string {
  return formatHumanTimestampTitle(createdAt);
}
