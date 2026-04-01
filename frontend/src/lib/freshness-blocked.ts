/**
 * Freshness-blocked reason parsing utilities.
 *
 * The Wave 1 backend encodes ExecutionBlocked reasons for freshness failures
 * using a structured pipe-delimited format:
 *   FRESHNESS_BLOCKED|{total_attempts}|{elapsed_minutes}|{file1,file2,...}|{message}
 */

export const FRESHNESS_BLOCKED_PREFIX = "FRESHNESS_BLOCKED|";

export interface FreshnessBlockedInfo {
  totalAttempts: number;
  elapsedMinutes: number;
  conflictFiles: string[];
  message: string;
}

/**
 * Parse a FRESHNESS_BLOCKED structured reason string.
 * Returns null if the string is not in the expected format.
 */
export function parseFreshnessBlockedReason(reason: string): FreshnessBlockedInfo | null {
  if (!reason.startsWith(FRESHNESS_BLOCKED_PREFIX)) return null;

  // Format: FRESHNESS_BLOCKED|{total_attempts}|{elapsed_minutes}|{file1,file2,...}|{message}
  const content = reason.slice(FRESHNESS_BLOCKED_PREFIX.length);
  const separatorIdx1 = content.indexOf("|");
  if (separatorIdx1 === -1) return null;
  const separatorIdx2 = content.indexOf("|", separatorIdx1 + 1);
  if (separatorIdx2 === -1) return null;
  const separatorIdx3 = content.indexOf("|", separatorIdx2 + 1);
  if (separatorIdx3 === -1) return null;

  const totalAttempts = parseInt(content.slice(0, separatorIdx1), 10);
  const elapsedMinutes = parseInt(content.slice(separatorIdx1 + 1, separatorIdx2), 10);
  const filesStr = content.slice(separatorIdx2 + 1, separatorIdx3);
  const message = content.slice(separatorIdx3 + 1);

  const conflictFiles = filesStr.split(",").filter(Boolean);

  return {
    totalAttempts: isNaN(totalAttempts) ? 0 : totalAttempts,
    elapsedMinutes: isNaN(elapsedMinutes) ? 0 : elapsedMinutes,
    conflictFiles,
    message,
  };
}
