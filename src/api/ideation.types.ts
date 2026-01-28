// Frontend types for ideation API responses (camelCase)

import type { IdeationSessionStatus } from "../types/ideation";

export interface IdeationSessionResponse {
  id: string;
  projectId: string;
  title: string | null;
  status: IdeationSessionStatus;
  planArtifactId: string | null;
  createdAt: string;
  updatedAt: string;
  archivedAt: string | null;
  convertedAt: string | null;
}

export interface TaskProposalResponse {
  id: string;
  sessionId: string;
  title: string;
  description: string | null;
  category: string;
  steps: string[];
  acceptanceCriteria: string[];
  suggestedPriority: string;
  priorityScore: number;
  priorityReason: string | null;
  estimatedComplexity: string;
  userPriority: string | null;
  userModified: boolean;
  status: string;
  selected: boolean;
  createdTaskId: string | null;
  planArtifactId: string | null;
  planVersionAtCreation: number | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface ChatMessageResponse {
  id: string;
  sessionId: string | null;
  projectId: string | null;
  taskId: string | null;
  role: string;
  content: string;
  metadata: string | null;
  parentMessageId: string | null;
  toolCalls: string | null;
  createdAt: string;
}

export interface SessionWithDataResponse {
  session: IdeationSessionResponse;
  proposals: TaskProposalResponse[];
  messages: ChatMessageResponse[];
}

export interface PriorityAssessmentResponse {
  proposalId: string;
  priority: string;
  score: number;
  reason: string;
}

export interface DependencyGraphNodeResponse {
  proposalId: string;
  title: string;
  inDegree: number;
  outDegree: number;
}

export interface DependencyGraphEdgeResponse {
  from: string;
  to: string;
}

export interface DependencyGraphResponse {
  nodes: DependencyGraphNodeResponse[];
  edges: DependencyGraphEdgeResponse[];
  criticalPath: string[];
  hasCycles: boolean;
  cycles: string[][] | null;
}

export interface ApplyProposalsResultResponse {
  createdTaskIds: string[];
  dependenciesCreated: number;
  warnings: string[];
  sessionConverted: boolean;
}

// Input types for API calls

export interface CreateProposalInput {
  sessionId: string;
  title: string;
  category: string;
  description?: string;
  steps?: string[];
  acceptanceCriteria?: string[];
  priority?: string;
  complexity?: string;
}

export interface UpdateProposalInput {
  title?: string;
  description?: string;
  category?: string;
  steps?: string[];
  acceptanceCriteria?: string[];
  userPriority?: string;
  complexity?: string;
}

export interface ApplyProposalsInput {
  sessionId: string;
  proposalIds: string[];
  targetColumn: string;
  preserveDependencies: boolean;
}
