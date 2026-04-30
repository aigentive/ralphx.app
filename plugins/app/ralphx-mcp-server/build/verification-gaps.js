export function isVerificationGapLike(value) {
    if (!value || typeof value !== "object" || Array.isArray(value)) {
        return false;
    }
    const gap = value;
    return (typeof gap.severity === "string" &&
        typeof gap.category === "string" &&
        typeof gap.description === "string");
}
export function mergeVerificationGaps(baseGaps, projectedGaps) {
    const merged = [];
    const seen = new Set();
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
export function hasBlockingVerificationGaps(gaps) {
    return gaps.some((gap) => gap.severity === "critical" ||
        gap.severity === "high" ||
        gap.severity === "medium");
}
//# sourceMappingURL=verification-gaps.js.map