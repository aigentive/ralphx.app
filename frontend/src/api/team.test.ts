/**
 * team API tests — Zod schema validation + API function tests
 */

import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  TeammateStatusSchema,
  TeamMessageSchema,
  TeamStatusSchema,
  TeammateSnapshotSchema,
  TeamSessionHistorySchema,
  TeamMessageRecordSchema,
  TeamHistoryResponseSchema,
  getTeamStatus,
  sendTeamMessage,
  sendTeammateMessage,
  getTeamMessages,
  stopTeammate,
  stopTeam,
  getTeammateCost,
} from "./team";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("team API schemas", () => {
  describe("TeammateStatusSchema", () => {
    it("parses valid teammate status", () => {
      const data = {
        name: "coder-1",
        color: "#3b82f6",
        model: "sonnet",
        role: "Auth middleware",
        status: "running",
        cost: {
          input_tokens: 50000,
          output_tokens: 10000,
          cache_creation_tokens: 2000,
          cache_read_tokens: 1000,
          estimated_usd: 0.30,
        },
        spawned_at: "2026-02-15T10:00:00Z",
        last_activity_at: "2026-02-15T10:05:00Z",
      };
      const result = TeammateStatusSchema.parse(data);
      expect(result.name).toBe("coder-1");
      expect(result.cost.estimated_usd).toBe(0.30);
    });

    it("rejects missing name", () => {
      const data = {
        color: "#3b82f6",
        model: "sonnet",
        role: "Auth",
        status: "running",
        cost: {
          input_tokens: 0,
          output_tokens: 0,
          cache_creation_tokens: 0,
          cache_read_tokens: 0,
          estimated_usd: 0,
        },
        spawned_at: "2026-02-15T10:00:00Z",
        last_activity_at: "2026-02-15T10:00:00Z",
      };
      expect(() => TeammateStatusSchema.parse(data)).toThrow();
    });
  });

  describe("TeamMessageSchema", () => {
    it("parses valid message", () => {
      const data = {
        id: "msg-1",
        sender: "coder-1",
        recipient: "coder-2",
        content: "Session type exported",
        message_type: "teammate_message",
        timestamp: "2026-02-15T10:00:00Z",
      };
      const result = TeamMessageSchema.parse(data);
      expect(result.sender).toBe("coder-1");
      expect(result.recipient).toBe("coder-2");
    });

    it("accepts null recipient", () => {
      const data = {
        id: "msg-2",
        sender: "coder-1",
        recipient: null,
        content: "Broadcast message",
        message_type: "broadcast",
        timestamp: "2026-02-15T10:00:00Z",
      };
      const result = TeamMessageSchema.parse(data);
      expect(result.recipient).toBeNull();
    });

    it("rejects missing fields", () => {
      expect(() => TeamMessageSchema.parse({ id: "msg-1" })).toThrow();
    });
  });

  describe("TeamStatusSchema", () => {
    it("parses valid team status", () => {
      const data = {
        name: "task-abc",
        context_type: "task_execution",
        context_id: "abc",
        lead_name: "lead-agent",
        teammates: [
          {
            name: "coder-1",
            color: "#3b82f6",
            model: "sonnet",
            role: "Auth",
            status: "running",
            cost: {
              input_tokens: 50000,
              output_tokens: 10000,
              cache_creation_tokens: 0,
              cache_read_tokens: 0,
              estimated_usd: 0.30,
            },
            spawned_at: "2026-02-15T10:00:00Z",
            last_activity_at: "2026-02-15T10:05:00Z",
          },
        ],
        phase: "active",
        created_at: "2026-02-15T10:00:00Z",
        message_count: 5,
      };
      const result = TeamStatusSchema.parse(data);
      expect(result.name).toBe("task-abc");
      expect(result.teammates).toHaveLength(1);
      expect(result.message_count).toBe(5);
    });

    it("accepts null lead_name", () => {
      const data = {
        name: "task-abc",
        context_type: "task_execution",
        context_id: "abc",
        lead_name: null,
        teammates: [],
        phase: "forming",
        created_at: "2026-02-15T10:00:00Z",
        message_count: 0,
      };
      const result = TeamStatusSchema.parse(data);
      expect(result.lead_name).toBeNull();
    });

    it("rejects invalid structure", () => {
      expect(() => TeamStatusSchema.parse({ name: "x" })).toThrow();
    });
  });

  // ── History Schemas (camelCase from backend) ─────────────────────────

  describe("TeammateSnapshotSchema", () => {
    it("parses camelCase snapshot from backend", () => {
      const data = {
        name: "coder-1",
        color: "#3b82f6",
        model: "sonnet",
        role: "Auth middleware",
        status: "shutdown",
        cost: {
          input_tokens: 50000,
          output_tokens: 10000,
          cache_creation_tokens: 2000,
          cache_read_tokens: 1000,
          estimated_usd: 0.30,
        },
        spawnedAt: "2026-02-15T10:00:00+00:00",
        lastActivityAt: "2026-02-15T10:05:00+00:00",
      };
      const result = TeammateSnapshotSchema.parse(data);
      expect(result.name).toBe("coder-1");
      expect(result.spawnedAt).toBe("2026-02-15T10:00:00+00:00");
    });

    it("rejects snake_case field names", () => {
      const data = {
        name: "coder-1",
        color: "#3b82f6",
        model: "sonnet",
        role: "Auth",
        status: "shutdown",
        cost: { input_tokens: 0, output_tokens: 0, cache_creation_tokens: 0, cache_read_tokens: 0, estimated_usd: 0 },
        spawned_at: "2026-02-15T10:00:00Z",
        last_activity_at: "2026-02-15T10:00:00Z",
      };
      expect(() => TeammateSnapshotSchema.parse(data)).toThrow();
    });
  });

  describe("TeamSessionHistorySchema", () => {
    it("parses camelCase session from backend", () => {
      const data = {
        id: "session-1",
        teamName: "task-abc",
        leadName: "team-lead",
        contextType: "task_execution",
        contextId: "abc",
        phase: "disbanded",
        createdAt: "2026-02-15T10:00:00+00:00",
        disbandedAt: "2026-02-15T11:00:00+00:00",
        teammates: [],
      };
      const result = TeamSessionHistorySchema.parse(data);
      expect(result.teamName).toBe("task-abc");
      expect(result.disbandedAt).toBe("2026-02-15T11:00:00+00:00");
    });

    it("accepts null leadName and disbandedAt", () => {
      const data = {
        id: "session-2",
        teamName: "task-xyz",
        leadName: null,
        contextType: "task_execution",
        contextId: "xyz",
        phase: "active",
        createdAt: "2026-02-15T10:00:00+00:00",
        disbandedAt: null,
        teammates: [],
      };
      const result = TeamSessionHistorySchema.parse(data);
      expect(result.leadName).toBeNull();
      expect(result.disbandedAt).toBeNull();
    });
  });

  describe("TeamMessageRecordSchema", () => {
    it("parses camelCase history message from backend", () => {
      const data = {
        id: "msg-1",
        sender: "coder-1",
        recipient: "lead",
        content: "Task done",
        messageType: "teammate_message",
        createdAt: "2026-02-15T10:00:00+00:00",
      };
      const result = TeamMessageRecordSchema.parse(data);
      expect(result.messageType).toBe("teammate_message");
      expect(result.createdAt).toBe("2026-02-15T10:00:00+00:00");
    });

    it("rejects snake_case field names", () => {
      const data = {
        id: "msg-1",
        sender: "coder-1",
        recipient: null,
        content: "Hello",
        message_type: "broadcast",
        timestamp: "2026-02-15T10:00:00Z",
      };
      expect(() => TeamMessageRecordSchema.parse(data)).toThrow();
    });
  });

  describe("TeamHistoryResponseSchema", () => {
    it("parses full history response", () => {
      const data = {
        session: {
          id: "session-1",
          teamName: "task-abc",
          leadName: "lead",
          contextType: "task_execution",
          contextId: "abc",
          phase: "disbanded",
          createdAt: "2026-02-15T10:00:00+00:00",
          disbandedAt: "2026-02-15T11:00:00+00:00",
          teammates: [{
            name: "coder-1",
            color: "#3b82f6",
            model: "sonnet",
            role: "Auth",
            status: "shutdown",
            cost: { input_tokens: 1000, output_tokens: 500, cache_creation_tokens: 0, cache_read_tokens: 0, estimated_usd: 0.05 },
            spawnedAt: "2026-02-15T10:00:00+00:00",
            lastActivityAt: "2026-02-15T10:30:00+00:00",
          }],
        },
        messages: [{
          id: "msg-1",
          sender: "coder-1",
          recipient: null,
          content: "Done",
          messageType: "teammate_message",
          createdAt: "2026-02-15T10:30:00+00:00",
        }],
      };
      const result = TeamHistoryResponseSchema.parse(data);
      expect(result.session?.teamName).toBe("task-abc");
      expect(result.messages).toHaveLength(1);
      expect(result.messages[0]!.createdAt).toBe("2026-02-15T10:30:00+00:00");
    });

    it("accepts null session with empty messages", () => {
      const data = { session: null, messages: [] };
      const result = TeamHistoryResponseSchema.parse(data);
      expect(result.session).toBeNull();
      expect(result.messages).toEqual([]);
    });
  });
});

// ============================================================================
// API Function Tests
// ============================================================================

const createMockTeamStatus = () => ({
  name: "task-abc",
  context_type: "task_execution",
  context_id: "abc",
  lead_name: "lead-agent",
  teammates: [],
  phase: "active",
  created_at: "2026-02-15T10:00:00Z",
  message_count: 3,
});

const createMockMessage = (overrides = {}) => ({
  id: "msg-1",
  sender: "coder-1",
  recipient: "coder-2",
  content: "Hello",
  message_type: "teammate_message",
  timestamp: "2026-02-15T10:00:00Z",
  ...overrides,
});

describe("team API functions", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("getTeamStatus", () => {
    it("returns parsed team status when result is non-null", async () => {
      const raw = createMockTeamStatus();
      mockInvoke.mockResolvedValue(raw);

      const result = await getTeamStatus("task-abc");

      expect(mockInvoke).toHaveBeenCalledWith("get_team_status", { teamName: "task-abc" });
      expect(result).not.toBeNull();
      expect(result!.name).toBe("task-abc");
      expect(result!.phase).toBe("active");
    });

    it("returns null when result is null", async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await getTeamStatus("nonexistent");

      expect(mockInvoke).toHaveBeenCalledWith("get_team_status", { teamName: "nonexistent" });
      expect(result).toBeNull();
    });
  });

  describe("sendTeamMessage", () => {
    it("wraps params in input object and parses response", async () => {
      const raw = createMockMessage();
      mockInvoke.mockResolvedValue(raw);

      const result = await sendTeamMessage("task-abc", "coder-2", "Hello");

      expect(mockInvoke).toHaveBeenCalledWith("send_team_message", {
        input: { teamName: "task-abc", target: "coder-2", content: "Hello" },
      });
      expect(result.sender).toBe("coder-1");
      expect(result.content).toBe("Hello");
    });
  });

  describe("sendTeammateMessage", () => {
    it("wraps params in input object for stdin routing", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await sendTeammateMessage("task-abc", "coder-1", "Hello teammate");

      expect(mockInvoke).toHaveBeenCalledWith("send_teammate_message", {
        input: { teamName: "task-abc", teammateName: "coder-1", content: "Hello teammate" },
      });
    });
  });

  describe("getTeamMessages", () => {
    it("fetches messages without limit", async () => {
      mockInvoke.mockResolvedValue([createMockMessage()]);

      const result = await getTeamMessages("task-abc");

      expect(mockInvoke).toHaveBeenCalledWith("get_team_messages", {
        teamName: "task-abc",
      });
      expect(result).toHaveLength(1);
    });

    it("includes limit when provided", async () => {
      mockInvoke.mockResolvedValue([createMockMessage()]);

      await getTeamMessages("task-abc", 10);

      expect(mockInvoke).toHaveBeenCalledWith("get_team_messages", {
        teamName: "task-abc",
        limit: 10,
      });
    });
  });

  describe("stopTeammate", () => {
    it("calls stop_teammate with correct params", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await stopTeammate("task-abc", "coder-1");

      expect(mockInvoke).toHaveBeenCalledWith("stop_teammate", {
        teamName: "task-abc",
        teammateName: "coder-1",
      });
    });
  });

  describe("stopTeam", () => {
    it("calls stop_team with team name", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await stopTeam("task-abc");

      expect(mockInvoke).toHaveBeenCalledWith("stop_team", { teamName: "task-abc" });
    });
  });

  describe("getTeammateCost", () => {
    it("parses inline schema response correctly", async () => {
      const raw = {
        teammate_name: "coder-1",
        input_tokens: 5000,
        output_tokens: 2000,
        cache_creation_tokens: 100,
        cache_read_tokens: 50,
        estimated_usd: 0.15,
      };
      mockInvoke.mockResolvedValue(raw);

      const result = await getTeammateCost("task-abc", "coder-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_teammate_cost", {
        teamName: "task-abc",
        teammateName: "coder-1",
      });
      expect(result.teammate_name).toBe("coder-1");
      expect(result.estimated_usd).toBe(0.15);
    });

    it("rejects invalid response shape", async () => {
      mockInvoke.mockResolvedValue({ invalid: true });

      await expect(getTeammateCost("task-abc", "coder-1")).rejects.toThrow();
    });
  });
});
