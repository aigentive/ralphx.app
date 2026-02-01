import { describe, it, expect } from "vitest";
import {
  IdeationSessionStatusSchema,
  IDEATION_SESSION_STATUS_VALUES,
  IdeationSessionSchema,
  PrioritySchema,
  PRIORITY_VALUES,
  ComplexitySchema,
  COMPLEXITY_VALUES,
  ProposalStatusSchema,
  PROPOSAL_STATUS_VALUES,
  TaskProposalSchema,
  MessageRoleSchema,
  MESSAGE_ROLE_VALUES,
  ChatMessageSchema,
  DependencyGraphNodeSchema,
  DependencyGraphEdgeSchema,
  DependencyGraphSchema,
  PriorityAssessmentSchema,
  ApplyProposalsInputSchema,
  ApplyProposalsResultSchema,
  CreateSessionInputSchema,
  CreateProposalInputSchema,
  UpdateProposalInputSchema,
  SendChatMessageInputSchema,
} from "./ideation";

describe("IdeationSessionStatusSchema", () => {
  it("should have 3 status values", () => {
    expect(IDEATION_SESSION_STATUS_VALUES.length).toBe(3);
  });

  it("should parse all valid statuses", () => {
    for (const status of IDEATION_SESSION_STATUS_VALUES) {
      expect(IdeationSessionStatusSchema.parse(status)).toBe(status);
    }
  });

  it("should include expected statuses", () => {
    expect(IDEATION_SESSION_STATUS_VALUES).toContain("active");
    expect(IDEATION_SESSION_STATUS_VALUES).toContain("archived");
    expect(IDEATION_SESSION_STATUS_VALUES).toContain("accepted");
  });

  it("should reject invalid status", () => {
    expect(() => IdeationSessionStatusSchema.parse("invalid")).toThrow();
    expect(() => IdeationSessionStatusSchema.parse("Active")).toThrow();
  });
});

describe("IdeationSessionSchema", () => {
  const validSession = {
    id: "session-123",
    projectId: "project-456",
    title: "Feature Planning",
    status: "active" as const,
    createdAt: "2026-01-24T12:00:00Z",
    updatedAt: "2026-01-24T12:00:00Z",
    archivedAt: null,
    convertedAt: null,
  };

  it("should parse a valid session", () => {
    expect(() => IdeationSessionSchema.parse(validSession)).not.toThrow();
  });

  it("should parse session with null title", () => {
    const sessionWithNullTitle = { ...validSession, title: null };
    expect(() => IdeationSessionSchema.parse(sessionWithNullTitle)).not.toThrow();
  });

  it("should parse session with timestamps", () => {
    const sessionWithTimestamps = {
      ...validSession,
      status: "archived" as const,
      archivedAt: "2026-01-24T13:00:00Z",
    };
    expect(() => IdeationSessionSchema.parse(sessionWithTimestamps)).not.toThrow();
  });

  it("should reject session with empty id", () => {
    expect(() => IdeationSessionSchema.parse({ ...validSession, id: "" })).toThrow();
  });

  it("should reject session with invalid status", () => {
    expect(() => IdeationSessionSchema.parse({ ...validSession, status: "invalid" })).toThrow();
  });
});

describe("PrioritySchema", () => {
  it("should have 4 priority values", () => {
    expect(PRIORITY_VALUES.length).toBe(4);
  });

  it("should parse all valid priorities", () => {
    for (const priority of PRIORITY_VALUES) {
      expect(PrioritySchema.parse(priority)).toBe(priority);
    }
  });

  it("should include expected priorities in order", () => {
    expect(PRIORITY_VALUES).toEqual(["critical", "high", "medium", "low"]);
  });

  it("should reject invalid priority", () => {
    expect(() => PrioritySchema.parse("invalid")).toThrow();
    expect(() => PrioritySchema.parse("High")).toThrow();
  });
});

describe("ComplexitySchema", () => {
  it("should have 5 complexity values", () => {
    expect(COMPLEXITY_VALUES.length).toBe(5);
  });

  it("should parse all valid complexities", () => {
    for (const complexity of COMPLEXITY_VALUES) {
      expect(ComplexitySchema.parse(complexity)).toBe(complexity);
    }
  });

  it("should include expected complexities in order", () => {
    expect(COMPLEXITY_VALUES).toEqual([
      "trivial",
      "simple",
      "moderate",
      "complex",
      "very_complex",
    ]);
  });

  it("should reject invalid complexity", () => {
    expect(() => ComplexitySchema.parse("invalid")).toThrow();
    expect(() => ComplexitySchema.parse("Moderate")).toThrow();
  });
});

describe("ProposalStatusSchema", () => {
  it("should have 4 status values", () => {
    expect(PROPOSAL_STATUS_VALUES.length).toBe(4);
  });

  it("should parse all valid statuses", () => {
    for (const status of PROPOSAL_STATUS_VALUES) {
      expect(ProposalStatusSchema.parse(status)).toBe(status);
    }
  });

  it("should include expected statuses", () => {
    expect(PROPOSAL_STATUS_VALUES).toContain("pending");
    expect(PROPOSAL_STATUS_VALUES).toContain("accepted");
    expect(PROPOSAL_STATUS_VALUES).toContain("rejected");
    expect(PROPOSAL_STATUS_VALUES).toContain("modified");
  });

  it("should reject invalid status", () => {
    expect(() => ProposalStatusSchema.parse("invalid")).toThrow();
  });
});

describe("TaskProposalSchema", () => {
  const validProposal = {
    id: "proposal-123",
    sessionId: "session-456",
    title: "Implement user authentication",
    description: "Add JWT-based authentication",
    category: "feature",
    steps: ["Setup auth database", "Implement JWT service"],
    acceptanceCriteria: ["Users can login", "Tokens expire after 24h"],
    suggestedPriority: "high" as const,
    priorityScore: 75,
    priorityReason: "High business value",
    estimatedComplexity: "moderate" as const,
    userPriority: null,
    userModified: false,
    status: "pending" as const,
    selected: true,
    createdTaskId: null,
    sortOrder: 0,
    createdAt: "2026-01-24T12:00:00Z",
    updatedAt: "2026-01-24T12:00:00Z",
  };

  it("should parse a valid proposal", () => {
    expect(() => TaskProposalSchema.parse(validProposal)).not.toThrow();
  });

  it("should parse proposal with null optional fields", () => {
    const minimalProposal = {
      ...validProposal,
      description: null,
      steps: [],
      acceptanceCriteria: [],
      priorityReason: null,
      userPriority: null,
      createdTaskId: null,
    };
    expect(() => TaskProposalSchema.parse(minimalProposal)).not.toThrow();
  });

  it("should parse proposal with user priority override", () => {
    const proposalWithOverride = {
      ...validProposal,
      userPriority: "critical" as const,
      userModified: true,
    };
    expect(() => TaskProposalSchema.parse(proposalWithOverride)).not.toThrow();
  });

  it("should parse proposal linked to a created task", () => {
    const linkedProposal = {
      ...validProposal,
      status: "accepted" as const,
      createdTaskId: "task-789",
    };
    expect(() => TaskProposalSchema.parse(linkedProposal)).not.toThrow();
  });

  it("should reject proposal with empty id", () => {
    expect(() => TaskProposalSchema.parse({ ...validProposal, id: "" })).toThrow();
  });

  it("should reject proposal with empty title", () => {
    expect(() => TaskProposalSchema.parse({ ...validProposal, title: "" })).toThrow();
  });

  it("should reject proposal with invalid priority", () => {
    expect(() =>
      TaskProposalSchema.parse({ ...validProposal, suggestedPriority: "invalid" })
    ).toThrow();
  });

  it("should reject proposal with priority score out of range", () => {
    expect(() =>
      TaskProposalSchema.parse({ ...validProposal, priorityScore: -1 })
    ).toThrow();
    expect(() =>
      TaskProposalSchema.parse({ ...validProposal, priorityScore: 101 })
    ).toThrow();
  });
});

describe("MessageRoleSchema", () => {
  it("should have 3 role values", () => {
    expect(MESSAGE_ROLE_VALUES.length).toBe(3);
  });

  it("should parse all valid roles", () => {
    for (const role of MESSAGE_ROLE_VALUES) {
      expect(MessageRoleSchema.parse(role)).toBe(role);
    }
  });

  it("should include expected roles", () => {
    expect(MESSAGE_ROLE_VALUES).toContain("user");
    expect(MESSAGE_ROLE_VALUES).toContain("orchestrator");
    expect(MESSAGE_ROLE_VALUES).toContain("system");
  });

  it("should reject invalid role", () => {
    expect(() => MessageRoleSchema.parse("assistant")).toThrow();
    expect(() => MessageRoleSchema.parse("User")).toThrow();
  });
});

describe("ChatMessageSchema", () => {
  const validMessage = {
    id: "msg-123",
    sessionId: "session-456",
    projectId: null,
    taskId: null,
    role: "user" as const,
    content: "I need to implement authentication",
    metadata: null,
    parentMessageId: null,
    createdAt: "2026-01-24T12:00:00Z",
  };

  it("should parse a valid message", () => {
    expect(() => ChatMessageSchema.parse(validMessage)).not.toThrow();
  });

  it("should parse message in session context", () => {
    expect(() => ChatMessageSchema.parse(validMessage)).not.toThrow();
    const result = ChatMessageSchema.parse(validMessage);
    expect(result.sessionId).toBe("session-456");
    expect(result.projectId).toBeNull();
  });

  it("should parse message in project context", () => {
    const projectMessage = {
      ...validMessage,
      sessionId: null,
      projectId: "project-123",
    };
    expect(() => ChatMessageSchema.parse(projectMessage)).not.toThrow();
  });

  it("should parse message about a task", () => {
    const taskMessage = {
      ...validMessage,
      sessionId: null,
      taskId: "task-123",
    };
    expect(() => ChatMessageSchema.parse(taskMessage)).not.toThrow();
  });

  it("should parse orchestrator message", () => {
    const orchestratorMessage = {
      ...validMessage,
      role: "orchestrator" as const,
      content: "I can help you with that. Let me suggest some tasks...",
    };
    expect(() => ChatMessageSchema.parse(orchestratorMessage)).not.toThrow();
  });

  it("should parse system message", () => {
    const systemMessage = {
      ...validMessage,
      role: "system" as const,
      content: "Session started",
    };
    expect(() => ChatMessageSchema.parse(systemMessage)).not.toThrow();
  });

  it("should parse message with metadata", () => {
    const messageWithMetadata = {
      ...validMessage,
      metadata: '{"key": "value"}',
    };
    expect(() => ChatMessageSchema.parse(messageWithMetadata)).not.toThrow();
  });

  it("should parse threaded message", () => {
    const threadedMessage = {
      ...validMessage,
      parentMessageId: "msg-100",
    };
    expect(() => ChatMessageSchema.parse(threadedMessage)).not.toThrow();
  });

  it("should reject message with empty id", () => {
    expect(() => ChatMessageSchema.parse({ ...validMessage, id: "" })).toThrow();
  });

  it("should reject message with empty content", () => {
    expect(() => ChatMessageSchema.parse({ ...validMessage, content: "" })).toThrow();
  });

  it("should reject message with invalid role", () => {
    expect(() => ChatMessageSchema.parse({ ...validMessage, role: "invalid" })).toThrow();
  });
});

describe("DependencyGraphSchema", () => {
  const validNode = {
    proposalId: "proposal-1",
    title: "Setup database",
    inDegree: 0,
    outDegree: 2,
  };

  const validEdge = {
    from: "proposal-2",
    to: "proposal-1",
  };

  const validGraph = {
    nodes: [validNode],
    edges: [validEdge],
    criticalPath: ["proposal-1", "proposal-2"],
    hasCycles: false,
    cycles: null,
  };

  it("should parse a valid node", () => {
    expect(() => DependencyGraphNodeSchema.parse(validNode)).not.toThrow();
  });

  it("should parse a valid edge", () => {
    expect(() => DependencyGraphEdgeSchema.parse(validEdge)).not.toThrow();
  });

  it("should parse a valid graph", () => {
    expect(() => DependencyGraphSchema.parse(validGraph)).not.toThrow();
  });

  it("should parse empty graph", () => {
    const emptyGraph = {
      nodes: [],
      edges: [],
      criticalPath: [],
      hasCycles: false,
      cycles: null,
    };
    expect(() => DependencyGraphSchema.parse(emptyGraph)).not.toThrow();
  });

  it("should parse graph with cycles", () => {
    const graphWithCycles = {
      ...validGraph,
      hasCycles: true,
      cycles: [["proposal-1", "proposal-2", "proposal-1"]],
    };
    expect(() => DependencyGraphSchema.parse(graphWithCycles)).not.toThrow();
  });

  it("should reject node with negative degree", () => {
    expect(() =>
      DependencyGraphNodeSchema.parse({ ...validNode, inDegree: -1 })
    ).toThrow();
  });
});

describe("PriorityAssessmentSchema", () => {
  const validAssessment = {
    proposalId: "proposal-123",
    priority: "high" as const,
    score: 75,
    reason: "High business value and blocking other tasks",
  };

  it("should parse a valid assessment", () => {
    expect(() => PriorityAssessmentSchema.parse(validAssessment)).not.toThrow();
  });

  it("should reject assessment with score out of range", () => {
    expect(() =>
      PriorityAssessmentSchema.parse({ ...validAssessment, score: -1 })
    ).toThrow();
    expect(() =>
      PriorityAssessmentSchema.parse({ ...validAssessment, score: 101 })
    ).toThrow();
  });
});

describe("ApplyProposalsInputSchema", () => {
  const validInput = {
    sessionId: "session-123",
    proposalIds: ["proposal-1", "proposal-2"],
    targetColumn: "backlog",
    preserveDependencies: true,
  };

  it("should parse valid input", () => {
    expect(() => ApplyProposalsInputSchema.parse(validInput)).not.toThrow();
  });

  it("should reject empty proposal list", () => {
    expect(() =>
      ApplyProposalsInputSchema.parse({ ...validInput, proposalIds: [] })
    ).toThrow();
  });

  it("should reject empty session id", () => {
    expect(() =>
      ApplyProposalsInputSchema.parse({ ...validInput, sessionId: "" })
    ).toThrow();
  });
});

describe("ApplyProposalsResultSchema", () => {
  const validResult = {
    createdTaskIds: ["task-1", "task-2"],
    dependenciesCreated: 1,
    warnings: ["Some dependency not preserved"],
    sessionConverted: false,
  };

  it("should parse valid result", () => {
    expect(() => ApplyProposalsResultSchema.parse(validResult)).not.toThrow();
  });

  it("should parse result with empty warnings", () => {
    const resultNoWarnings = { ...validResult, warnings: [] };
    expect(() => ApplyProposalsResultSchema.parse(resultNoWarnings)).not.toThrow();
  });

  it("should parse result with session converted", () => {
    const convertedResult = { ...validResult, sessionConverted: true };
    expect(() => ApplyProposalsResultSchema.parse(convertedResult)).not.toThrow();
  });
});

describe("CreateSessionInputSchema", () => {
  it("should parse input with project id only", () => {
    const input = { projectId: "project-123" };
    expect(() => CreateSessionInputSchema.parse(input)).not.toThrow();
  });

  it("should parse input with title", () => {
    const input = { projectId: "project-123", title: "Feature Planning" };
    const result = CreateSessionInputSchema.parse(input);
    expect(result.title).toBe("Feature Planning");
  });

  it("should reject empty project id", () => {
    expect(() => CreateSessionInputSchema.parse({ projectId: "" })).toThrow();
  });
});

describe("CreateProposalInputSchema", () => {
  const validInput = {
    sessionId: "session-123",
    title: "Implement feature",
    category: "feature",
  };

  it("should parse minimal input", () => {
    expect(() => CreateProposalInputSchema.parse(validInput)).not.toThrow();
  });

  it("should parse input with all optional fields", () => {
    const fullInput = {
      ...validInput,
      description: "Detailed description",
      steps: ["Step 1", "Step 2"],
      acceptanceCriteria: ["Criterion 1"],
      priority: "high",
      complexity: "moderate",
    };
    expect(() => CreateProposalInputSchema.parse(fullInput)).not.toThrow();
  });

  it("should reject empty session id", () => {
    expect(() =>
      CreateProposalInputSchema.parse({ ...validInput, sessionId: "" })
    ).toThrow();
  });

  it("should reject empty title", () => {
    expect(() =>
      CreateProposalInputSchema.parse({ ...validInput, title: "" })
    ).toThrow();
  });
});

describe("UpdateProposalInputSchema", () => {
  it("should parse empty update (no changes)", () => {
    expect(() => UpdateProposalInputSchema.parse({})).not.toThrow();
  });

  it("should parse title update", () => {
    expect(() => UpdateProposalInputSchema.parse({ title: "New Title" })).not.toThrow();
  });

  it("should parse multiple field updates", () => {
    const update = {
      title: "Updated Title",
      description: "Updated description",
      userPriority: "critical",
    };
    expect(() => UpdateProposalInputSchema.parse(update)).not.toThrow();
  });

  it("should reject empty title", () => {
    expect(() => UpdateProposalInputSchema.parse({ title: "" })).toThrow();
  });
});

describe("SendChatMessageInputSchema", () => {
  it("should parse message to session", () => {
    const input = {
      sessionId: "session-123",
      role: "user",
      content: "Hello",
    };
    expect(() => SendChatMessageInputSchema.parse(input)).not.toThrow();
  });

  it("should parse message to project", () => {
    const input = {
      projectId: "project-123",
      role: "user",
      content: "Hello",
    };
    expect(() => SendChatMessageInputSchema.parse(input)).not.toThrow();
  });

  it("should parse message about task", () => {
    const input = {
      taskId: "task-123",
      role: "user",
      content: "Hello",
    };
    expect(() => SendChatMessageInputSchema.parse(input)).not.toThrow();
  });

  it("should parse message with metadata", () => {
    const input = {
      sessionId: "session-123",
      role: "user",
      content: "Hello",
      metadata: '{"key": "value"}',
    };
    expect(() => SendChatMessageInputSchema.parse(input)).not.toThrow();
  });

  it("should parse threaded message", () => {
    const input = {
      sessionId: "session-123",
      role: "user",
      content: "Hello",
      parentMessageId: "msg-100",
    };
    expect(() => SendChatMessageInputSchema.parse(input)).not.toThrow();
  });

  it("should reject message with empty content", () => {
    expect(() =>
      SendChatMessageInputSchema.parse({
        sessionId: "session-123",
        role: "user",
        content: "",
      })
    ).toThrow();
  });

  it("should reject message with invalid role", () => {
    expect(() =>
      SendChatMessageInputSchema.parse({
        sessionId: "session-123",
        role: "invalid",
        content: "Hello",
      })
    ).toThrow();
  });
});
