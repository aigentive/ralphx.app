/**
 * Mock Ideation API
 *
 * Mirrors the interface of src/api/ideation.ts with mock implementations.
 */

import type {
  IdeationSessionResponse,
  TaskProposalResponse,
  SessionWithDataResponse,
  PriorityAssessmentResponse,
  DependencyGraphResponse,
  ApplyProposalsResultResponse,
  CreateProposalInput,
  UpdateProposalInput,
  ApplyProposalsInput,
} from "@/api/ideation.types";
import type { IdeationSettings, IdeationPlanMode } from "@/types/ideation-config";
import { generateTestUuid } from "@/test/mock-data";

// ============================================================================
// Mock State
// ============================================================================

const mockSessions: Map<string, IdeationSessionResponse> = new Map();
const mockProposals: Map<string, TaskProposalResponse> = new Map();
const mockDependencies: Map<string, Set<string>> = new Map();

// Initialize with some mock data
function ensureMockData(): void {
  if (mockSessions.size > 0) return;

  const session: IdeationSessionResponse = {
    id: "session-mock-1",
    projectId: "project-mock-1",
    title: "Demo Ideation Session",
    status: "active",
    planArtifactId: null,
    seedTaskId: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    archivedAt: null,
    convertedAt: null,
  };
  mockSessions.set(session.id, session);

  const proposal: TaskProposalResponse = {
    id: "proposal-mock-1",
    sessionId: session.id,
    title: "Sample Proposal",
    description: "A sample proposal for testing",
    category: "feature",
    steps: ["Step 1", "Step 2", "Step 3"],
    acceptanceCriteria: ["Criteria 1", "Criteria 2"],
    suggestedPriority: "medium",
    priorityScore: 50,
    priorityReason: "Medium complexity feature",
    estimatedComplexity: "medium",
    userPriority: null,
    userModified: false,
    status: "pending",
    createdTaskId: null,
    planArtifactId: null,
    planVersionAtCreation: null,
    sortOrder: 0,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
  mockProposals.set(proposal.id, proposal);
}

// ============================================================================
// Mock Ideation API
// ============================================================================

export const mockIdeationApi = {
  sessions: {
    create: async (
      projectId: string,
      title?: string,
      seedTaskId?: string
    ): Promise<IdeationSessionResponse> => {
      const session: IdeationSessionResponse = {
        id: generateTestUuid(),
        projectId,
        title: title ?? null,
        status: "active",
        planArtifactId: null,
        seedTaskId: seedTaskId ?? null,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        archivedAt: null,
        convertedAt: null,
      };
      mockSessions.set(session.id, session);
      return session;
    },

    get: async (sessionId: string): Promise<IdeationSessionResponse | null> => {
      ensureMockData();
      return mockSessions.get(sessionId) ?? null;
    },

    getWithData: async (sessionId: string): Promise<SessionWithDataResponse | null> => {
      ensureMockData();
      const session = mockSessions.get(sessionId);
      if (!session) return null;

      const proposals = Array.from(mockProposals.values()).filter(
        (p) => p.sessionId === sessionId
      );

      return {
        session,
        proposals,
        messages: [],
      };
    },

    list: async (projectId: string): Promise<IdeationSessionResponse[]> => {
      ensureMockData();
      return Array.from(mockSessions.values()).filter(
        (s) => s.projectId === projectId
      );
    },

    archive: async (_sessionId: string): Promise<void> => {
      // No-op in read-only mode
    },

    delete: async (_sessionId: string): Promise<void> => {
      // No-op in read-only mode
    },

    updateTitle: async (
      sessionId: string,
      title: string | null
    ): Promise<IdeationSessionResponse> => {
      ensureMockData();
      const session = mockSessions.get(sessionId);
      if (!session) {
        throw new Error(`Session not found: ${sessionId}`);
      }
      return { ...session, title, updatedAt: new Date().toISOString() };
    },

    spawnSessionNamer: async (
      _sessionId: string,
      _firstMessage: string
    ): Promise<void> => {
      // No-op in mock mode
    },

    spawnDependencySuggester: async (_sessionId: string): Promise<void> => {
      // No-op in mock mode
    },
  },

  proposals: {
    create: async (input: CreateProposalInput): Promise<TaskProposalResponse> => {
      const proposal: TaskProposalResponse = {
        id: generateTestUuid(),
        sessionId: input.sessionId,
        title: input.title,
        description: input.description ?? null,
        category: input.category,
        steps: input.steps ?? [],
        acceptanceCriteria: input.acceptanceCriteria ?? [],
        suggestedPriority: input.priority ?? "medium",
        priorityScore: 50,
        priorityReason: null,
        estimatedComplexity: input.complexity ?? "medium",
        userPriority: null,
        userModified: false,
        status: "pending",
        createdTaskId: null,
        planArtifactId: null,
        planVersionAtCreation: null,
        sortOrder: mockProposals.size,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      mockProposals.set(proposal.id, proposal);
      return proposal;
    },

    get: async (proposalId: string): Promise<TaskProposalResponse | null> => {
      ensureMockData();
      return mockProposals.get(proposalId) ?? null;
    },

    list: async (sessionId: string): Promise<TaskProposalResponse[]> => {
      ensureMockData();
      return Array.from(mockProposals.values()).filter(
        (p) => p.sessionId === sessionId
      );
    },

    update: async (
      proposalId: string,
      input: UpdateProposalInput
    ): Promise<TaskProposalResponse> => {
      ensureMockData();
      const existing = mockProposals.get(proposalId);
      if (!existing) {
        throw new Error(`Proposal not found: ${proposalId}`);
      }
      const updated: TaskProposalResponse = {
        ...existing,
        userModified: true,
        updatedAt: new Date().toISOString(),
      };
      if (input.title !== undefined) updated.title = input.title;
      if (input.description !== undefined) updated.description = input.description;
      if (input.category !== undefined) updated.category = input.category;
      if (input.steps !== undefined) updated.steps = input.steps;
      if (input.acceptanceCriteria !== undefined) updated.acceptanceCriteria = input.acceptanceCriteria;
      if (input.userPriority !== undefined) updated.userPriority = input.userPriority;
      if (input.complexity !== undefined) updated.estimatedComplexity = input.complexity;
      return updated;
    },

    delete: async (_proposalId: string): Promise<void> => {
      // No-op in read-only mode
    },

    reorder: async (_sessionId: string, _proposalIds: string[]): Promise<void> => {
      // No-op in read-only mode
    },

    assessPriority: async (proposalId: string): Promise<PriorityAssessmentResponse> => {
      return {
        proposalId,
        priority: "medium",
        score: 50,
        reason: "Mock priority assessment",
      };
    },

    assessAllPriorities: async (
      sessionId: string
    ): Promise<PriorityAssessmentResponse[]> => {
      ensureMockData();
      const proposals = Array.from(mockProposals.values()).filter(
        (p) => p.sessionId === sessionId
      );
      return proposals.map((p) => ({
        proposalId: p.id,
        priority: "medium",
        score: 50,
        reason: "Mock priority assessment",
      }));
    },
  },

  dependencies: {
    add: async (_proposalId: string, _dependsOnId: string): Promise<void> => {
      // No-op in read-only mode
    },

    remove: async (_proposalId: string, _dependsOnId: string): Promise<void> => {
      // No-op in read-only mode
    },

    getDependencies: async (proposalId: string): Promise<string[]> => {
      const deps = mockDependencies.get(proposalId);
      return deps ? Array.from(deps) : [];
    },

    getDependents: async (_proposalId: string): Promise<string[]> => {
      return [];
    },

    analyze: async (sessionId: string): Promise<DependencyGraphResponse> => {
      ensureMockData();
      const proposals = Array.from(mockProposals.values()).filter(
        (p) => p.sessionId === sessionId
      );
      return {
        nodes: proposals.map((p) => ({
          proposalId: p.id,
          title: p.title,
          inDegree: 0,
          outDegree: 0,
        })),
        edges: [],
        criticalPath: [],
        hasCycles: false,
        cycles: null,
      };
    },
  },

  apply: {
    toKanban: async (_input: ApplyProposalsInput): Promise<ApplyProposalsResultResponse> => {
      return {
        createdTaskIds: [],
        dependenciesCreated: 0,
        warnings: ["Mock mode: proposals not actually applied"],
        sessionConverted: false,
      };
    },
  },

  taskDependencies: {
    getBlockers: async (_taskId: string): Promise<string[]> => {
      return [];
    },

    getBlocked: async (_taskId: string): Promise<string[]> => {
      return [];
    },
  },

  settings: {
    get: async (): Promise<IdeationSettings> => {
      return {
        planMode: "optional" as IdeationPlanMode,
        requirePlanApproval: false,
        suggestPlansForComplex: true,
        autoLinkProposals: true,
      };
    },

    update: async (settings: IdeationSettings): Promise<IdeationSettings> => {
      return settings;
    },
  },
} as const;
