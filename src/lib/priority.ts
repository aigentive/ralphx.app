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
 * - 90-100: critical
 * - 70-89: high
 * - 40-69: medium
 * - 0-39: low
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
