/**
 * Priority utility functions
 *
 * Maps numeric priority scores (0-100) to Priority enum levels
 */

import type { Priority } from "@/types/ideation";

/**
 * Convert numeric priority score to Priority enum
 *
 * Mapping:
 * - >= 90: critical
 * - >= 70: high
 * - >= 40: medium
 * - < 40: low
 *
 * @param score - Numeric priority score (0-100)
 * @returns Priority level
 */
export function priorityFromScore(score: number): Priority {
  if (score >= 90) return "critical";
  if (score >= 70) return "high";
  if (score >= 40) return "medium";
  return "low";
}
