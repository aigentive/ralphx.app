export type VerificationGapLike = {
    severity: string;
    category: string;
    description: string;
    why_it_matters?: unknown;
    [key: string]: unknown;
};
export declare function isVerificationGapLike(value: unknown): value is VerificationGapLike;
export declare function mergeVerificationGaps(baseGaps: unknown[], projectedGaps: unknown[]): VerificationGapLike[];
export declare function hasBlockingVerificationGaps(gaps: VerificationGapLike[]): boolean;
//# sourceMappingURL=verification-gaps.d.ts.map