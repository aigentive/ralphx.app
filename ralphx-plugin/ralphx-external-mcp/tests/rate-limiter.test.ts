import { describe, it, expect, beforeEach } from "vitest";
import { RateLimiter } from "../src/rate-limiter.js";

describe("RateLimiter — token bucket per key", () => {
  let limiter: RateLimiter;

  beforeEach(() => {
    limiter = new RateLimiter({ requestsPerSecond: 2, authFailuresBeforeLockout: 3, lockoutDurationMs: 100 });
  });

  it("allows requests up to burst capacity", () => {
    expect(limiter.checkKey("key-1")).toBe(true);
    expect(limiter.checkKey("key-1")).toBe(true);
  });

  it("blocks requests after burst is exhausted", () => {
    limiter.checkKey("key-1"); // token 1
    limiter.checkKey("key-1"); // token 2
    expect(limiter.checkKey("key-1")).toBe(false);
  });

  it("refills tokens over time", async () => {
    limiter.checkKey("key-1");
    limiter.checkKey("key-1");
    expect(limiter.checkKey("key-1")).toBe(false);

    // Wait 600ms — enough to refill 1+ token at 2 req/s
    await new Promise((r) => setTimeout(r, 600));
    expect(limiter.checkKey("key-1")).toBe(true);
  });

  it("tracks separate buckets per key", () => {
    limiter.checkKey("key-A");
    limiter.checkKey("key-A");
    expect(limiter.checkKey("key-A")).toBe(false);

    // key-B has its own bucket — still allowed
    expect(limiter.checkKey("key-B")).toBe(true);
  });
});

describe("RateLimiter — IP auth throttle", () => {
  let limiter: RateLimiter;

  beforeEach(() => {
    limiter = new RateLimiter({
      requestsPerSecond: 10,
      authFailuresBeforeLockout: 3,
      lockoutDurationMs: 100,
    });
  });

  it("allows auth by default", () => {
    expect(limiter.checkIpAuth("1.2.3.4")).toBe(true);
  });

  it("blocks IP after threshold failures", () => {
    limiter.recordAuthFailure("1.2.3.4");
    limiter.recordAuthFailure("1.2.3.4");
    expect(limiter.checkIpAuth("1.2.3.4")).toBe(true); // 2 failures, not yet locked

    limiter.recordAuthFailure("1.2.3.4"); // 3rd failure → lockout
    expect(limiter.checkIpAuth("1.2.3.4")).toBe(false);
  });

  it("unblocks IP after lockout duration expires", async () => {
    limiter.recordAuthFailure("5.6.7.8");
    limiter.recordAuthFailure("5.6.7.8");
    limiter.recordAuthFailure("5.6.7.8");
    expect(limiter.checkIpAuth("5.6.7.8")).toBe(false);

    await new Promise((r) => setTimeout(r, 150));
    expect(limiter.checkIpAuth("5.6.7.8")).toBe(true);
  });

  it("resets failure count on successful auth", () => {
    limiter.recordAuthFailure("9.9.9.9");
    limiter.recordAuthFailure("9.9.9.9");
    limiter.recordAuthSuccess("9.9.9.9");

    limiter.recordAuthFailure("9.9.9.9");
    limiter.recordAuthFailure("9.9.9.9");
    // Only 2 failures after reset — still allowed
    expect(limiter.checkIpAuth("9.9.9.9")).toBe(true);
  });

  it("does not affect other IPs", () => {
    limiter.recordAuthFailure("a.b.c.d");
    limiter.recordAuthFailure("a.b.c.d");
    limiter.recordAuthFailure("a.b.c.d");
    expect(limiter.checkIpAuth("a.b.c.d")).toBe(false);
    expect(limiter.checkIpAuth("e.f.g.h")).toBe(true);
  });

  it("getLockoutRemainingMs returns 0 when not locked", () => {
    expect(limiter.getLockoutRemainingMs("clean-ip")).toBe(0);
  });

  it("getLockoutRemainingMs returns positive ms when locked", () => {
    limiter.recordAuthFailure("lock-me");
    limiter.recordAuthFailure("lock-me");
    limiter.recordAuthFailure("lock-me");
    expect(limiter.getLockoutRemainingMs("lock-me")).toBeGreaterThan(0);
  });
});
