export type VerificationGapLike = {
  severity: string;
  category: string;
  description: string;
  why_it_matters?: unknown;
  [key: string]: unknown;
};

export function isVerificationGapLike(value: unknown): value is VerificationGapLike {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return false;
  }
  const gap = value as Record<string, unknown>;
  return (
    typeof gap.severity === "string" &&
    typeof gap.category === "string" &&
    typeof gap.description === "string"
  );
}

export function mergeVerificationGaps(
  baseGaps: unknown[],
  projectedGaps: unknown[]
): VerificationGapLike[] {
  const merged: VerificationGapLike[] = [];
  const seen = new Set<string>();
  for (const gap of [...baseGaps, ...projectedGaps]) {
    if (!isVerificationGapLike(gap)) {
      continue;
    }
    const key = [
      gap.severity,
      gap.category,
      gap.description,
      typeof gap.why_it_matters === "string" ? gap.why_it_matters : "",
    ].join("\u0000");
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    merged.push(gap);
  }
  return merged;
}

export function hasBlockingVerificationGaps(gaps: VerificationGapLike[]): boolean {
  return gaps.some(
    (gap) =>
      gap.severity === "critical" ||
      gap.severity === "high" ||
      gap.severity === "medium"
  );
}
