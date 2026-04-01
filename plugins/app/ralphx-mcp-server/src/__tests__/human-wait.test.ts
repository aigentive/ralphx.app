import { afterEach, describe, expect, it, vi } from "vitest";

import {
  createHumanWaitAbortController,
  HUMAN_WAIT_CLIENT_TIMEOUT_MS,
  isHumanWaitTimeoutError,
} from "../human-wait.js";

describe("human-wait", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("aborts the controller at the configured timeout", () => {
    vi.useFakeTimers();

    const { controller, timeoutId } = createHumanWaitAbortController(1_000);
    expect(controller.signal.aborted).toBe(false);

    vi.advanceTimersByTime(1_000);

    expect(controller.signal.aborted).toBe(true);
    clearTimeout(timeoutId);
  });

  it("treats AbortError as a timeout", () => {
    const error = new Error("This operation was aborted");
    error.name = "AbortError";

    expect(isHumanWaitTimeoutError(error, 10_000)).toBe(true);
  });

  it("treats near-deadline fetch failures as timeouts", () => {
    const error = new Error("fetch failed");

    expect(
      isHumanWaitTimeoutError(
        error,
        HUMAN_WAIT_CLIENT_TIMEOUT_MS - 5_000,
        HUMAN_WAIT_CLIENT_TIMEOUT_MS
      )
    ).toBe(true);
  });

  it("does not treat early fetch failures as timeouts", () => {
    const error = new Error("fetch failed");

    expect(
      isHumanWaitTimeoutError(
        error,
        30_000,
        HUMAN_WAIT_CLIENT_TIMEOUT_MS
      )
    ).toBe(false);
  });
});
