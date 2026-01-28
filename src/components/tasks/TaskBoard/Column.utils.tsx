/**
 * Column utilities - helper functions for column grouping
 *
 * Extracted from Column.tsx to keep component under LOC limits.
 */

import { type ReactNode } from "react";
import { RotateCcw, RefreshCw, Clock, Bot, CheckCircle } from "lucide-react";

/** Storage key prefix for collapsed group state */
export const COLLAPSED_GROUPS_KEY = "ralphx:collapsed-groups";

/**
 * Get collapsed group state from localStorage
 *
 * @param columnId - The column ID
 * @returns Set of collapsed group IDs
 */
export function getCollapsedGroups(columnId: string): Set<string> {
  try {
    const stored = localStorage.getItem(`${COLLAPSED_GROUPS_KEY}:${columnId}`);
    if (stored) {
      const parsed = JSON.parse(stored);
      if (Array.isArray(parsed)) {
        return new Set(parsed);
      }
    }
  } catch {
    // Ignore parse errors
  }
  return new Set();
}

/**
 * Save collapsed group state to localStorage
 *
 * @param columnId - The column ID
 * @param collapsed - Set of collapsed group IDs
 */
export function saveCollapsedGroups(
  columnId: string,
  collapsed: Set<string>
): void {
  try {
    localStorage.setItem(
      `${COLLAPSED_GROUPS_KEY}:${columnId}`,
      JSON.stringify(Array.from(collapsed))
    );
  } catch {
    // Ignore storage errors
  }
}

/**
 * Map icon name to Lucide icon component
 *
 * Supports icons used in StateGroup definitions.
 *
 * @param iconName - Lucide icon name (e.g., "RotateCcw", "Clock")
 * @returns React node for the icon, or null if not found
 */
export function getGroupIcon(iconName: string | undefined): ReactNode {
  if (!iconName) return null;

  // Icons must be rendered as JSX, so we return createElement results
  // Using direct imports to avoid dynamic import complexity
  const iconMap: Record<string, () => ReactNode> = {
    RotateCcw: () => <RotateCcw className="w-3 h-3" />,
    RefreshCw: () => <RefreshCw className="w-3 h-3" />,
    Clock: () => <Clock className="w-3 h-3" />,
    Bot: () => <Bot className="w-3 h-3" />,
    CheckCircle: () => <CheckCircle className="w-3 h-3" />,
  };

  const iconFactory = iconMap[iconName];
  return iconFactory ? iconFactory() : null;
}
