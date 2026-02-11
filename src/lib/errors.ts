/**
 * Extract a human-readable error message from an unknown caught value.
 *
 * Precedence:
 * 1. Error objects → .message
 * 2. String values → trimmed
 * 3. Plain objects with .message / .error / .cause.message
 * 4. Serializable objects → JSON.stringify
 * 5. Fallback string
 */
export function extractErrorMessage(
  error: unknown,
  fallback: string,
): string {
  // 1. Error instance
  if (error instanceof Error) {
    const msg = error.message.trim();
    return msg || fallback;
  }

  // 2. String
  if (typeof error === "string") {
    const msg = error.trim();
    return msg || fallback;
  }

  // 3. Plain object forms
  if (error != null && typeof error === "object") {
    const obj = error as Record<string, unknown>;

    if (typeof obj.message === "string") {
      const msg = obj.message.trim();
      if (msg) return msg;
    }

    if (typeof obj.error === "string") {
      const msg = obj.error.trim();
      if (msg) return msg;
    }

    const cause = obj.cause;
    if (cause != null && typeof cause === "object") {
      const causeMsg = (cause as Record<string, unknown>).message;
      if (typeof causeMsg === "string") {
        const msg = causeMsg.trim();
        if (msg) return msg;
      }
    }

    // 4. JSON serializable
    try {
      const json = JSON.stringify(error);
      if (json && json !== "{}") return json;
    } catch {
      // Circular or non-serializable — fall through
    }
  }

  // 5. Fallback
  return fallback;
}
