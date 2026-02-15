/**
 * team API tests — Zod schema validation tests
 */

import { describe, it, expect } from "vitest";
import { TeammateStatusSchema, TeamMessageSchema, TeamStatusSchema } from "./team";

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
});
