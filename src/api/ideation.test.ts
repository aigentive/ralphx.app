import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { ideationApi } from "./ideation";

// Cast invoke to a mock function for testing
const mockInvoke = invoke as ReturnType<typeof vi.fn>;

// Helper to create mock ideation session (snake_case - matches Rust backend)
const createMockSessionRaw = (overrides = {}) => ({
  id: "session-1",
  project_id: "project-1",
  title: null,
  status: "active",
  created_at: "2026-01-24T12:00:00Z",
  updated_at: "2026-01-24T12:00:00Z",
  archived_at: null,
  converted_at: null,
  ...overrides,
});

// Helper to create mock task proposal (snake_case - matches Rust backend)
const createMockProposalRaw = (overrides = {}) => ({
  id: "proposal-1",
  session_id: "session-1",
  title: "Test Proposal",
  description: null,
  category: "feature",
  steps: [],
  acceptance_criteria: [],
  suggested_priority: "medium",
  priority_score: 50,
  priority_reason: null,
  estimated_complexity: "moderate",
  user_priority: null,
  user_modified: false,
  status: "pending",
  created_task_id: null,
  sort_order: 0,
  created_at: "2026-01-24T12:00:00Z",
  updated_at: "2026-01-24T12:00:00Z",
  ...overrides,
});

// Helper to create mock chat message (snake_case - matches Rust backend)
const createMockMessageRaw = (overrides = {}) => ({
  id: "message-1",
  session_id: "session-1",
  project_id: null,
  task_id: null,
  role: "user",
  content: "Hello",
  metadata: null,
  parent_message_id: null,
  created_at: "2026-01-24T12:00:00Z",
  ...overrides,
});

describe("ideationApi.sessions", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("create", () => {
    it("should call create_ideation_session with project_id and title", async () => {
      const session = createMockSessionRaw({ title: "My Session" });
      mockInvoke.mockResolvedValue(session);

      await ideationApi.sessions.create("project-1", "My Session");

      expect(mockInvoke).toHaveBeenCalledWith("create_ideation_session", {
        input: { project_id: "project-1", title: "My Session" },
      });
    });

    it("should call create_ideation_session with just project_id", async () => {
      const session = createMockSessionRaw();
      mockInvoke.mockResolvedValue(session);

      await ideationApi.sessions.create("project-1");

      expect(mockInvoke).toHaveBeenCalledWith("create_ideation_session", {
        input: { project_id: "project-1", title: undefined },
      });
    });

    it("should return created session", async () => {
      const session = createMockSessionRaw({ title: "New Session" });
      mockInvoke.mockResolvedValue(session);

      const result = await ideationApi.sessions.create("project-1", "New Session");

      expect(result.title).toBe("New Session");
      expect(result.status).toBe("active");
    });

    it("should validate session schema", async () => {
      mockInvoke.mockResolvedValue({ invalid: "session" });

      await expect(ideationApi.sessions.create("project-1")).rejects.toThrow();
    });
  });

  describe("get", () => {
    it("should call get_ideation_session with id", async () => {
      const session = createMockSessionRaw();
      mockInvoke.mockResolvedValue(session);

      await ideationApi.sessions.get("session-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_ideation_session", {
        id: "session-1",
      });
    });

    it("should return session when found", async () => {
      const session = createMockSessionRaw({ title: "Found Session" });
      mockInvoke.mockResolvedValue(session);

      const result = await ideationApi.sessions.get("session-1");

      expect(result?.title).toBe("Found Session");
    });

    it("should return null when not found", async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await ideationApi.sessions.get("nonexistent");

      expect(result).toBeNull();
    });
  });

  describe("getWithData", () => {
    it("should call get_ideation_session_with_data with id", async () => {
      const data = {
        session: createMockSessionRaw(),
        proposals: [],
        messages: [],
      };
      mockInvoke.mockResolvedValue(data);

      await ideationApi.sessions.getWithData("session-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_ideation_session_with_data", {
        id: "session-1",
      });
    });

    it("should return session with proposals and messages", async () => {
      const data = {
        session: createMockSessionRaw(),
        proposals: [createMockProposalRaw()],
        messages: [createMockMessageRaw()],
      };
      mockInvoke.mockResolvedValue(data);

      const result = await ideationApi.sessions.getWithData("session-1");

      expect(result?.proposals).toHaveLength(1);
      expect(result?.messages).toHaveLength(1);
    });

    it("should return null when session not found", async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await ideationApi.sessions.getWithData("nonexistent");

      expect(result).toBeNull();
    });
  });

  describe("list", () => {
    it("should call list_ideation_sessions with project_id", async () => {
      mockInvoke.mockResolvedValue([createMockSessionRaw()]);

      await ideationApi.sessions.list("project-1");

      expect(mockInvoke).toHaveBeenCalledWith("list_ideation_sessions", {
        project_id: "project-1",
      });
    });

    it("should return array of sessions", async () => {
      const sessions = [
        createMockSessionRaw({ id: "s1" }),
        createMockSessionRaw({ id: "s2", title: "Session 2" }),
      ];
      mockInvoke.mockResolvedValue(sessions);

      const result = await ideationApi.sessions.list("project-1");

      expect(result).toHaveLength(2);
      expect(result[0]?.id).toBe("s1");
      expect(result[1]?.title).toBe("Session 2");
    });

    it("should return empty array when no sessions", async () => {
      mockInvoke.mockResolvedValue([]);

      const result = await ideationApi.sessions.list("project-1");

      expect(result).toEqual([]);
    });
  });

  describe("archive", () => {
    it("should call archive_ideation_session with id", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await ideationApi.sessions.archive("session-1");

      expect(mockInvoke).toHaveBeenCalledWith("archive_ideation_session", {
        id: "session-1",
      });
    });

    it("should propagate errors", async () => {
      mockInvoke.mockRejectedValue(new Error("Session not found"));

      await expect(ideationApi.sessions.archive("nonexistent")).rejects.toThrow(
        "Session not found"
      );
    });
  });

  describe("delete", () => {
    it("should call delete_ideation_session with id", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await ideationApi.sessions.delete("session-1");

      expect(mockInvoke).toHaveBeenCalledWith("delete_ideation_session", {
        id: "session-1",
      });
    });

    it("should propagate errors", async () => {
      mockInvoke.mockRejectedValue(new Error("Session not found"));

      await expect(ideationApi.sessions.delete("nonexistent")).rejects.toThrow(
        "Session not found"
      );
    });
  });
});

describe("ideationApi.proposals", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("create", () => {
    it("should call create_task_proposal with input", async () => {
      const proposal = createMockProposalRaw();
      mockInvoke.mockResolvedValue(proposal);

      await ideationApi.proposals.create({
        sessionId: "session-1",
        title: "New Feature",
        category: "feature",
      });

      expect(mockInvoke).toHaveBeenCalledWith("create_task_proposal", {
        input: {
          session_id: "session-1",
          title: "New Feature",
          category: "feature",
          description: undefined,
          steps: undefined,
          acceptance_criteria: undefined,
          priority: undefined,
          complexity: undefined,
        },
      });
    });

    it("should pass all optional fields", async () => {
      const proposal = createMockProposalRaw();
      mockInvoke.mockResolvedValue(proposal);

      await ideationApi.proposals.create({
        sessionId: "session-1",
        title: "New Feature",
        category: "feature",
        description: "A description",
        steps: ["Step 1", "Step 2"],
        acceptanceCriteria: ["AC1"],
        priority: "high",
        complexity: "complex",
      });

      expect(mockInvoke).toHaveBeenCalledWith("create_task_proposal", {
        input: {
          session_id: "session-1",
          title: "New Feature",
          category: "feature",
          description: "A description",
          steps: ["Step 1", "Step 2"],
          acceptance_criteria: ["AC1"],
          priority: "high",
          complexity: "complex",
        },
      });
    });

    it("should return created proposal", async () => {
      const proposal = createMockProposalRaw({ title: "Created Proposal" });
      mockInvoke.mockResolvedValue(proposal);

      const result = await ideationApi.proposals.create({
        sessionId: "session-1",
        title: "Created Proposal",
        category: "feature",
      });

      expect(result.title).toBe("Created Proposal");
    });
  });

  describe("get", () => {
    it("should call get_task_proposal with id", async () => {
      const proposal = createMockProposalRaw();
      mockInvoke.mockResolvedValue(proposal);

      await ideationApi.proposals.get("proposal-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_task_proposal", {
        id: "proposal-1",
      });
    });

    it("should return null when not found", async () => {
      mockInvoke.mockResolvedValue(null);

      const result = await ideationApi.proposals.get("nonexistent");

      expect(result).toBeNull();
    });
  });

  describe("list", () => {
    it("should call list_session_proposals with session_id", async () => {
      mockInvoke.mockResolvedValue([createMockProposalRaw()]);

      await ideationApi.proposals.list("session-1");

      expect(mockInvoke).toHaveBeenCalledWith("list_session_proposals", {
        session_id: "session-1",
      });
    });

    it("should return array of proposals", async () => {
      const proposals = [
        createMockProposalRaw({ id: "p1" }),
        createMockProposalRaw({ id: "p2", title: "Proposal 2" }),
      ];
      mockInvoke.mockResolvedValue(proposals);

      const result = await ideationApi.proposals.list("session-1");

      expect(result).toHaveLength(2);
    });
  });

  describe("update", () => {
    it("should call update_task_proposal with id and input", async () => {
      const proposal = createMockProposalRaw({ title: "Updated" });
      mockInvoke.mockResolvedValue(proposal);

      await ideationApi.proposals.update("proposal-1", { title: "Updated" });

      expect(mockInvoke).toHaveBeenCalledWith("update_task_proposal", {
        id: "proposal-1",
        input: {
          title: "Updated",
          description: undefined,
          category: undefined,
          steps: undefined,
          acceptance_criteria: undefined,
          user_priority: undefined,
          complexity: undefined,
        },
      });
    });

    it("should return updated proposal", async () => {
      const proposal = createMockProposalRaw({ title: "Updated Title", user_modified: true });
      mockInvoke.mockResolvedValue(proposal);

      const result = await ideationApi.proposals.update("proposal-1", {
        title: "Updated Title",
      });

      expect(result.title).toBe("Updated Title");
      expect(result.userModified).toBe(true);
    });
  });

  describe("delete", () => {
    it("should call delete_task_proposal with id", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await ideationApi.proposals.delete("proposal-1");

      expect(mockInvoke).toHaveBeenCalledWith("delete_task_proposal", {
        id: "proposal-1",
      });
    });
  });

  describe("reorder", () => {
    it("should call reorder_proposals with session_id and proposal_ids", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await ideationApi.proposals.reorder("session-1", ["p1", "p2", "p3"]);

      expect(mockInvoke).toHaveBeenCalledWith("reorder_proposals", {
        session_id: "session-1",
        proposal_ids: ["p1", "p2", "p3"],
      });
    });
  });

  describe("assessPriority", () => {
    it("should call assess_proposal_priority with id", async () => {
      mockInvoke.mockResolvedValue({
        proposal_id: "proposal-1",
        priority: "high",
        score: 75,
        reason: "Blocks 2 tasks",
      });

      const result = await ideationApi.proposals.assessPriority("proposal-1");

      expect(mockInvoke).toHaveBeenCalledWith("assess_proposal_priority", {
        id: "proposal-1",
      });
      expect(result.priority).toBe("high");
      expect(result.score).toBe(75);
    });
  });

  describe("assessAllPriorities", () => {
    it("should call assess_all_priorities with session_id", async () => {
      mockInvoke.mockResolvedValue([
        { proposal_id: "p1", priority: "high", score: 80, reason: "Reason 1" },
        { proposal_id: "p2", priority: "low", score: 30, reason: "Reason 2" },
      ]);

      const result = await ideationApi.proposals.assessAllPriorities("session-1");

      expect(mockInvoke).toHaveBeenCalledWith("assess_all_priorities", {
        session_id: "session-1",
      });
      expect(result).toHaveLength(2);
    });
  });
});

describe("ideationApi.dependencies", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("add", () => {
    it("should call add_proposal_dependency with proposal_id and depends_on_id", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await ideationApi.dependencies.add("proposal-1", "proposal-2");

      expect(mockInvoke).toHaveBeenCalledWith("add_proposal_dependency", {
        proposal_id: "proposal-1",
        depends_on_id: "proposal-2",
      });
    });
  });

  describe("remove", () => {
    it("should call remove_proposal_dependency", async () => {
      mockInvoke.mockResolvedValue(undefined);

      await ideationApi.dependencies.remove("proposal-1", "proposal-2");

      expect(mockInvoke).toHaveBeenCalledWith("remove_proposal_dependency", {
        proposal_id: "proposal-1",
        depends_on_id: "proposal-2",
      });
    });
  });

  describe("getDependencies", () => {
    it("should call get_proposal_dependencies", async () => {
      mockInvoke.mockResolvedValue(["p2", "p3"]);

      const result = await ideationApi.dependencies.getDependencies("proposal-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_proposal_dependencies", {
        proposal_id: "proposal-1",
      });
      expect(result).toEqual(["p2", "p3"]);
    });
  });

  describe("getDependents", () => {
    it("should call get_proposal_dependents", async () => {
      mockInvoke.mockResolvedValue(["p4", "p5"]);

      const result = await ideationApi.dependencies.getDependents("proposal-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_proposal_dependents", {
        proposal_id: "proposal-1",
      });
      expect(result).toEqual(["p4", "p5"]);
    });
  });

  describe("analyze", () => {
    it("should call analyze_dependencies with session_id", async () => {
      const graph = {
        nodes: [{ proposal_id: "p1", title: "P1", in_degree: 0, out_degree: 1 }],
        edges: [{ from: "p1", to: "p2" }],
        critical_path: ["p1", "p2"],
        has_cycles: false,
        cycles: null,
      };
      mockInvoke.mockResolvedValue(graph);

      const result = await ideationApi.dependencies.analyze("session-1");

      expect(mockInvoke).toHaveBeenCalledWith("analyze_dependencies", {
        session_id: "session-1",
      });
      expect(result.hasCycles).toBe(false);
      expect(result.criticalPath).toEqual(["p1", "p2"]);
    });

    it("should handle cycles", async () => {
      const graph = {
        nodes: [],
        edges: [],
        critical_path: [],
        has_cycles: true,
        cycles: [["p1", "p2", "p3"]],
      };
      mockInvoke.mockResolvedValue(graph);

      const result = await ideationApi.dependencies.analyze("session-1");

      expect(result.hasCycles).toBe(true);
      expect(result.cycles).toEqual([["p1", "p2", "p3"]]);
    });
  });
});

describe("ideationApi.apply", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("toKanban", () => {
    it("should call apply_proposals_to_kanban with input", async () => {
      mockInvoke.mockResolvedValue({
        created_task_ids: ["task-1", "task-2"],
        dependencies_created: 1,
        warnings: [],
        session_converted: false,
      });

      await ideationApi.apply.toKanban({
        sessionId: "session-1",
        proposalIds: ["p1", "p2"],
        targetColumn: "backlog",
        preserveDependencies: true,
      });

      expect(mockInvoke).toHaveBeenCalledWith("apply_proposals_to_kanban", {
        input: {
          session_id: "session-1",
          proposal_ids: ["p1", "p2"],
          target_column: "backlog",
          preserve_dependencies: true,
        },
      });
    });

    it("should return apply result", async () => {
      mockInvoke.mockResolvedValue({
        created_task_ids: ["task-1", "task-2"],
        dependencies_created: 1,
        warnings: ["Some dep not preserved"],
        session_converted: true,
      });

      const result = await ideationApi.apply.toKanban({
        sessionId: "session-1",
        proposalIds: ["p1", "p2"],
        targetColumn: "todo",
        preserveDependencies: true,
      });

      expect(result.createdTaskIds).toEqual(["task-1", "task-2"]);
      expect(result.dependenciesCreated).toBe(1);
      expect(result.warnings).toHaveLength(1);
      expect(result.sessionConverted).toBe(true);
    });
  });
});

describe("ideationApi.taskDependencies", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  describe("getBlockers", () => {
    it("should call get_task_blockers with task_id", async () => {
      mockInvoke.mockResolvedValue(["task-2", "task-3"]);

      const result = await ideationApi.taskDependencies.getBlockers("task-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_task_blockers", {
        task_id: "task-1",
      });
      expect(result).toEqual(["task-2", "task-3"]);
    });
  });

  describe("getBlocked", () => {
    it("should call get_blocked_tasks with task_id", async () => {
      mockInvoke.mockResolvedValue(["task-4"]);

      const result = await ideationApi.taskDependencies.getBlocked("task-1");

      expect(mockInvoke).toHaveBeenCalledWith("get_blocked_tasks", {
        task_id: "task-1",
      });
      expect(result).toEqual(["task-4"]);
    });
  });
});
