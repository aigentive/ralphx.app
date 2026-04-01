/**
 * MessageItem.utils - Utility functions for MessageItem
 */

/**
 * Format a timestamp for display in chat messages
 * Shows relative time for recent messages, absolute time for older ones
 */
export function formatTimestamp(createdAt: string): string {
  const date = new Date(createdAt);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;

  return date.toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });
}
