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
        current_activity: "Writing auth.ts",
        tokens_used: 50000,
        estimated_cost_usd: 0.30,
      };
      const result = TeammateStatusSchema.parse(data);
      expect(result.name).toBe("coder-1");
      expect(result.current_activity).toBe("Writing auth.ts");
    });

    it("accepts null current_activity", () => {
      const data = {
        name: "coder-1",
        color: "#3b82f6",
        model: "sonnet",
        role: "Auth middleware",
        status: "idle",
        current_activity: null,
        tokens_used: 0,
        estimated_cost_usd: 0,
      };
      const result = TeammateStatusSchema.parse(data);
      expect(result.current_activity).toBeNull();
    });

    it("rejects missing name", () => {
      const data = {
        color: "#3b82f6",
        model: "sonnet",
        role: "Auth",
        status: "running",
        current_activity: null,
        tokens_used: 0,
        estimated_cost_usd: 0,
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
        timestamp: "2026-02-15T10:00:00Z",
      };
      const result = TeamMessageSchema.parse(data);
      expect(result.sender).toBe("coder-1");
      expect(result.recipient).toBe("coder-2");
    });

    it("rejects missing fields", () => {
      expect(() => TeamMessageSchema.parse({ id: "msg-1" })).toThrow();
    });
  });

  describe("TeamStatusSchema", () => {
    it("parses valid team status", () => {
      const data = {
        team_name: "task-abc",
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
            current_activity: null,
            tokens_used: 50000,
            estimated_cost_usd: 0.30,
          },
        ],
        messages: [],
        total_tokens: 50000,
        estimated_cost_usd: 0.30,
        created_at: "2026-02-15T10:00:00Z",
      };
      const result = TeamStatusSchema.parse(data);
      expect(result.team_name).toBe("task-abc");
      expect(result.teammates).toHaveLength(1);
      expect(result.messages).toHaveLength(0);
    });

    it("parses with messages", () => {
      const data = {
        team_name: "task-abc",
        context_type: "task_execution",
        context_id: "abc",
        lead_name: "lead-agent",
        teammates: [],
        messages: [
          { id: "m1", sender: "a", recipient: "b", content: "hi", timestamp: "2026-02-15T10:00:00Z" },
        ],
        total_tokens: 0,
        estimated_cost_usd: 0,
        created_at: "2026-02-15T10:00:00Z",
      };
      const result = TeamStatusSchema.parse(data);
      expect(result.messages).toHaveLength(1);
    });

    it("rejects invalid structure", () => {
      expect(() => TeamStatusSchema.parse({ team_name: "x" })).toThrow();
    });
  });
});
