/**
 * Rate limiter for ralphx-external-mcp
 *
 * - Token bucket per API key (default: 10 req/s)
 * - IP-based auth throttle: 5 consecutive failures → 30s lockout
 */
const DEFAULT_OPTIONS = {
    requestsPerSecond: 10,
    authFailuresBeforeLockout: 5,
    lockoutDurationMs: 30_000,
};
export class RateLimiter {
    opts;
    keyBuckets = new Map();
    ipFailures = new Map();
    constructor(options = {}) {
        this.opts = { ...DEFAULT_OPTIONS, ...options };
    }
    /**
     * Check rate limit for an API key.
     * Returns true if the request is allowed, false if rate limited.
     */
    checkKey(keyId) {
        const now = Date.now();
        let bucket = this.keyBuckets.get(keyId);
        if (!bucket) {
            bucket = {
                tokens: this.opts.requestsPerSecond,
                lastRefillAt: now,
            };
            this.keyBuckets.set(keyId, bucket);
        }
        // Refill tokens based on elapsed time
        const elapsedSecs = (now - bucket.lastRefillAt) / 1000;
        const refill = elapsedSecs * this.opts.requestsPerSecond;
        bucket.tokens = Math.min(this.opts.requestsPerSecond, bucket.tokens + refill);
        bucket.lastRefillAt = now;
        if (bucket.tokens < 1) {
            return false;
        }
        bucket.tokens -= 1;
        return true;
    }
    /**
     * Check IP auth throttle.
     * Returns true if the IP is allowed to attempt auth, false if locked out.
     */
    checkIpAuth(ip) {
        const now = Date.now();
        const state = this.ipFailures.get(ip);
        if (!state)
            return true;
        if (state.lockedUntil !== null) {
            if (now < state.lockedUntil) {
                return false;
            }
            // Lockout expired — reset
            state.lockedUntil = null;
            state.failureCount = 0;
        }
        return true;
    }
    /**
     * Record a failed auth attempt from an IP.
     * If failure count reaches threshold, lock the IP.
     */
    recordAuthFailure(ip) {
        const now = Date.now();
        let state = this.ipFailures.get(ip);
        if (!state) {
            state = { failureCount: 0, lockedUntil: null };
            this.ipFailures.set(ip, state);
        }
        state.failureCount += 1;
        if (state.failureCount >= this.opts.authFailuresBeforeLockout) {
            state.lockedUntil = now + this.opts.lockoutDurationMs;
        }
    }
    /**
     * Record a successful auth for an IP (resets failure count).
     */
    recordAuthSuccess(ip) {
        this.ipFailures.delete(ip);
    }
    /**
     * Get remaining seconds of lockout for an IP (0 if not locked).
     */
    getLockoutRemainingMs(ip) {
        const now = Date.now();
        const state = this.ipFailures.get(ip);
        if (!state || state.lockedUntil === null)
            return 0;
        return Math.max(0, state.lockedUntil - now);
    }
    /** Clear all state (for testing) */
    reset() {
        this.keyBuckets.clear();
        this.ipFailures.clear();
    }
}
/** Singleton instance for use in server middleware */
let _instance = null;
export function getRateLimiter() {
    if (!_instance) {
        _instance = new RateLimiter();
    }
    return _instance;
}
export function configureRateLimiter(options) {
    _instance = new RateLimiter(options);
}
//# sourceMappingURL=rate-limiter.js.map