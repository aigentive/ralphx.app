import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { planApi } from "./plan";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("plan api", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it("gets active plan when set", async () => {
    mockInvoke.mockResolvedValue("session-123");

    const result = await planApi.getActivePlan("project-1");

    expect(mockInvoke).toHaveBeenCalledWith("get_active_plan", {
      projectId: "project-1",
    });
    expect(result).toBe("session-123");
  });

  it("gets null when no active plan", async () => {
    mockInvoke.mockResolvedValue(null);

    const result = await planApi.getActivePlan("project-1");

    expect(mockInvoke).toHaveBeenCalledWith("get_active_plan", {
      projectId: "project-1",
    });
    expect(result).toBeNull();
  });

  it("sets active plan with source tracking", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await planApi.setActivePlan("project-1", "session-123", "kanban_inline");

    expect(mockInvoke).toHaveBeenCalledWith("set_active_plan", {
      projectId: "project-1",
      ideationSessionId: "session-123",
      source: "kanban_inline",
    });
  });

  it("sets active plan with different sources", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await planApi.setActivePlan("project-1", "session-123", "graph_inline");
    await planApi.setActivePlan("project-2", "session-456", "quick_switcher");
    await planApi.setActivePlan("project-3", "session-789", "ideation");

    expect(mockInvoke).toHaveBeenNthCalledWith(1, "set_active_plan", {
      projectId: "project-1",
      ideationSessionId: "session-123",
      source: "graph_inline",
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "set_active_plan", {
      projectId: "project-2",
      ideationSessionId: "session-456",
      source: "quick_switcher",
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(3, "set_active_plan", {
      projectId: "project-3",
      ideationSessionId: "session-789",
      source: "ideation",
    });
  });

  it("clears active plan", async () => {
    mockInvoke.mockResolvedValue(undefined);

    await planApi.clearActivePlan("project-1");

    expect(mockInvoke).toHaveBeenCalledWith("clear_active_plan", {
      projectId: "project-1",
    });
  });

  it("handles errors from backend", async () => {
    mockInvoke.mockRejectedValue(new Error("Session not found"));

    await expect(
      planApi.setActivePlan("project-1", "invalid-session", "kanban_inline")
    ).rejects.toThrow("Session not found");
  });
});
