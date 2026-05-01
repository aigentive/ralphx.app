import React from "react";
import { render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { UpdateChecker } from "./UpdateChecker";

const mocks = vi.hoisted(() => ({
  check: vi.fn(),
  toast: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: (...args: unknown[]) => mocks.check(...args),
}));

vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: Object.assign(mocks.toast, {
    dismiss: vi.fn(),
    error: vi.fn(),
    loading: vi.fn(),
    success: vi.fn(),
  }),
}));

const update = {
  version: "0.3.2",
  currentVersion: "0.3.1",
  body: "Daily release",
  downloadAndInstall: vi.fn(),
};

describe("UpdateChecker", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mocks.check.mockReset();
    mocks.toast.mockReset();
    mocks.check.mockResolvedValue(update);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("checks after startup in React StrictMode", async () => {
    render(
      <React.StrictMode>
        <UpdateChecker />
      </React.StrictMode>,
    );

    expect(mocks.check).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(3_000);

    expect(mocks.check).toHaveBeenCalledTimes(1);
    expect(mocks.toast).toHaveBeenCalledTimes(1);
  });

  it("polls for later releases without re-notifying the same version", async () => {
    render(<UpdateChecker />);

    await vi.advanceTimersByTimeAsync(3_000);
    await vi.advanceTimersByTimeAsync(30 * 60 * 1_000);

    expect(mocks.check).toHaveBeenCalledTimes(2);
    expect(mocks.toast).toHaveBeenCalledTimes(1);
  });
});
