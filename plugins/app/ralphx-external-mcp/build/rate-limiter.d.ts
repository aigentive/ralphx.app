/**
 * Rate limiter for ralphx-external-mcp
 *
 * - Token bucket per API key (default: 10 req/s)
 * - IP-based auth throttle: 5 consecutive failures → 30s lockout
 */
export interface RateLimiterOptions {
    requestsPerSecond: number;
    authFailuresBeforeLockout: number;
    lockoutDurationMs: number;
}
export declare class RateLimiter {
    private readonly opts;
    private readonly keyBuckets;
    private readonly ipFailures;
    constructor(options?: Partial<RateLimiterOptions>);
    /**
     * Check rate limit for an API key.
     * Returns true if the request is allowed, false if rate limited.
     */
    checkKey(keyId: string): boolean;
    /**
     * Check IP auth throttle.
     * Returns true if the IP is allowed to attempt auth, false if locked out.
     */
    checkIpAuth(ip: string): boolean;
    /**
     * Record a failed auth attempt from an IP.
     * If failure count reaches threshold, lock the IP.
     */
    recordAuthFailure(ip: string): void;
    /**
     * Record a successful auth for an IP (resets failure count).
     */
    recordAuthSuccess(ip: string): void;
    /**
     * Get remaining seconds of lockout for an IP (0 if not locked).
     */
    getLockoutRemainingMs(ip: string): number;
    /** Clear all state (for testing) */
    reset(): void;
}
export declare function getRateLimiter(): RateLimiter;
export declare function configureRateLimiter(options: Partial<RateLimiterOptions>): void;
//# sourceMappingURL=rate-limiter.d.ts.map