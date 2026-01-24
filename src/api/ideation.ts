// Tauri invoke wrappers for ideation system with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Response Schemas (matching Rust backend serialization with snake_case)
// ============================================================================

/**
 * Ideation session response schema (snake_case from Rust)
 */
const IdeationSessionResponseSchema = z.object({
  id: z.string(),
  project_id: z.string(),
  title: z.string().nullable(),
  status: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
  archived_at: z.string().nullable(),
  converted_at: z.string().nullable(),
});

/**
 * Task proposal response schema (snake_case from Rust)
 */
const TaskProposalResponseSchema = z.object({
  id: z.string(),
  session_id: z.string(),
  title: z.string(),
  description: z.string().nullable(),
  category: z.string(),
  steps: z.array(z.string()),
  acceptance_criteria: z.array(z.string()),
  suggested_priority: z.string(),
  priority_score: z.number(),
  priority_reason: z.string().nullable(),
  estimated_complexity: z.string(),
  user_priority: z.string().nullable(),
  user_modified: z.boolean(),
  status: z.string(),
  selected: z.boolean(),
  created_task_id: z.string().nullable(),
  sort_order: z.number(),
  created_at: z.string(),
  updated_at: z.string(),
});

/**
 * Chat message response schema (snake_case from Rust)
 */
const ChatMessageResponseSchema = z.object({
  id: z.string(),
  session_id: z.string().nullable(),
  project_id: z.string().nullable(),
  task_id: z.string().nullable(),
  role: z.string(),
  content: z.string(),
  metadata: z.string().nullable(),
  parent_message_id: z.string().nullable(),
  created_at: z.string(),
});

/**
 * Session with proposals and messages (snake_case from Rust)
 */
const SessionWithDataResponseSchema = z.object({
  session: IdeationSessionResponseSchema,
  proposals: z.array(TaskProposalResponseSchema),
  messages: z.array(ChatMessageResponseSchema),
});

/**
 * Priority assessment response (snake_case from Rust)
 */
const PriorityAssessmentResponseSchema = z.object({
  proposal_id: z.string(),
  priority: z.string(),
  score: z.number(),
  reason: z.string(),
});

/**
 * Dependency graph node response (snake_case from Rust)
 */
const DependencyGraphNodeResponseSchema = z.object({
  proposal_id: z.string(),
  title: z.string(),
  in_degree: z.number(),
  out_degree: z.number(),
});

/**
 * Dependency graph edge response (snake_case from Rust)
 */
const DependencyGraphEdgeResponseSchema = z.object({
  from: z.string(),
  to: z.string(),
});

/**
 * Dependency graph response (snake_case from Rust)
 */
const DependencyGraphResponseSchema = z.object({
  nodes: z.array(DependencyGraphNodeResponseSchema),
  edges: z.array(DependencyGraphEdgeResponseSchema),
  critical_path: z.array(z.string()),
  has_cycles: z.boolean(),
  cycles: z.array(z.array(z.string())).nullable(),
});

/**
 * Apply proposals result response (snake_case from Rust)
 */
const ApplyProposalsResultResponseSchema = z.object({
  created_task_ids: z.array(z.string()),
  dependencies_created: z.number(),
  warnings: z.array(z.string()),
  session_converted: z.boolean(),
});

// ============================================================================
// Transformed Types (camelCase for frontend usage)
// ============================================================================

export interface IdeationSessionResponse {
  id: string;
  projectId: string;
  title: string | null;
  status: string;
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

// ============================================================================
// Input Types
// ============================================================================

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

// ============================================================================
// Transform Functions (snake_case -> camelCase)
// ============================================================================

function transformSession(raw: z.infer<typeof IdeationSessionResponseSchema>): IdeationSessionResponse {
  return {
    id: raw.id,
    projectId: raw.project_id,
    title: raw.title,
    status: raw.status,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
    archivedAt: raw.archived_at,
    convertedAt: raw.converted_at,
  };
}

function transformProposal(raw: z.infer<typeof TaskProposalResponseSchema>): TaskProposalResponse {
  return {
    id: raw.id,
    sessionId: raw.session_id,
    title: raw.title,
    description: raw.description,
    category: raw.category,
    steps: raw.steps,
    acceptanceCriteria: raw.acceptance_criteria,
    suggestedPriority: raw.suggested_priority,
    priorityScore: raw.priority_score,
    priorityReason: raw.priority_reason,
    estimatedComplexity: raw.estimated_complexity,
    userPriority: raw.user_priority,
    userModified: raw.user_modified,
    status: raw.status,
    selected: raw.selected,
    createdTaskId: raw.created_task_id,
    sortOrder: raw.sort_order,
    createdAt: raw.created_at,
    updatedAt: raw.updated_at,
  };
}

function transformMessage(raw: z.infer<typeof ChatMessageResponseSchema>): ChatMessageResponse {
  return {
    id: raw.id,
    sessionId: raw.session_id,
    projectId: raw.project_id,
    taskId: raw.task_id,
    role: raw.role,
    content: raw.content,
    metadata: raw.metadata,
    parentMessageId: raw.parent_message_id,
    createdAt: raw.created_at,
  };
}

function transformSessionWithData(raw: z.infer<typeof SessionWithDataResponseSchema>): SessionWithDataResponse {
  return {
    session: transformSession(raw.session),
    proposals: raw.proposals.map(transformProposal),
    messages: raw.messages.map(transformMessage),
  };
}

function transformPriorityAssessment(raw: z.infer<typeof PriorityAssessmentResponseSchema>): PriorityAssessmentResponse {
  return {
    proposalId: raw.proposal_id,
    priority: raw.priority,
    score: raw.score,
    reason: raw.reason,
  };
}

function transformDependencyGraph(raw: z.infer<typeof DependencyGraphResponseSchema>): DependencyGraphResponse {
  return {
    nodes: raw.nodes.map((n) => ({
      proposalId: n.proposal_id,
      title: n.title,
      inDegree: n.in_degree,
      outDegree: n.out_degree,
    })),
    edges: raw.edges,
    criticalPath: raw.critical_path,
    hasCycles: raw.has_cycles,
    cycles: raw.cycles,
  };
}

function transformApplyResult(raw: z.infer<typeof ApplyProposalsResultResponseSchema>): ApplyProposalsResultResponse {
  return {
    createdTaskIds: raw.created_task_ids,
    dependenciesCreated: raw.dependencies_created,
    warnings: raw.warnings,
    sessionConverted: raw.session_converted,
  };
}

// ============================================================================
// Typed Invoke Helpers
// ============================================================================

async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// ============================================================================
// API Object
// ============================================================================

/**
 * Ideation API wrappers for Tauri commands
 */
export const ideationApi = {
  /**
   * Session management operations
   */
  sessions: {
    /**
     * Create a new ideation session
     * @param projectId The project ID
     * @param title Optional session title
     * @returns The created session
     */
    create: async (projectId: string, title?: string): Promise<IdeationSessionResponse> => {
      const raw = await typedInvoke(
        "create_ideation_session",
        { input: { project_id: projectId, title } },
        IdeationSessionResponseSchema
      );
      return transformSession(raw);
    },

    /**
     * Get a session by ID
     * @param sessionId The session ID
     * @returns The session or null if not found
     */
    get: async (sessionId: string): Promise<IdeationSessionResponse | null> => {
      const raw = await typedInvoke(
        "get_ideation_session",
        { id: sessionId },
        IdeationSessionResponseSchema.nullable()
      );
      return raw ? transformSession(raw) : null;
    },

    /**
     * Get a session with all proposals and messages
     * @param sessionId The session ID
     * @returns Session with data or null if not found
     */
    getWithData: async (sessionId: string): Promise<SessionWithDataResponse | null> => {
      const raw = await typedInvoke(
        "get_ideation_session_with_data",
        { id: sessionId },
        SessionWithDataResponseSchema.nullable()
      );
      return raw ? transformSessionWithData(raw) : null;
    },

    /**
     * List all sessions for a project
     * @param projectId The project ID
     * @returns Array of sessions
     */
    list: async (projectId: string): Promise<IdeationSessionResponse[]> => {
      const raw = await typedInvoke(
        "list_ideation_sessions",
        { project_id: projectId },
        z.array(IdeationSessionResponseSchema)
      );
      return raw.map(transformSession);
    },

    /**
     * Archive a session
     * @param sessionId The session ID
     */
    archive: async (sessionId: string): Promise<void> => {
      await invoke("archive_ideation_session", { id: sessionId });
    },

    /**
     * Delete a session
     * @param sessionId The session ID
     */
    delete: async (sessionId: string): Promise<void> => {
      await invoke("delete_ideation_session", { id: sessionId });
    },
  },

  /**
   * Task proposal operations
   */
  proposals: {
    /**
     * Create a new task proposal
     * @param input Proposal creation data
     * @returns The created proposal
     */
    create: async (input: CreateProposalInput): Promise<TaskProposalResponse> => {
      const raw = await typedInvoke(
        "create_task_proposal",
        {
          input: {
            session_id: input.sessionId,
            title: input.title,
            category: input.category,
            description: input.description,
            steps: input.steps,
            acceptance_criteria: input.acceptanceCriteria,
            priority: input.priority,
            complexity: input.complexity,
          },
        },
        TaskProposalResponseSchema
      );
      return transformProposal(raw);
    },

    /**
     * Get a proposal by ID
     * @param proposalId The proposal ID
     * @returns The proposal or null if not found
     */
    get: async (proposalId: string): Promise<TaskProposalResponse | null> => {
      const raw = await typedInvoke(
        "get_task_proposal",
        { id: proposalId },
        TaskProposalResponseSchema.nullable()
      );
      return raw ? transformProposal(raw) : null;
    },

    /**
     * List all proposals for a session
     * @param sessionId The session ID
     * @returns Array of proposals
     */
    list: async (sessionId: string): Promise<TaskProposalResponse[]> => {
      const raw = await typedInvoke(
        "list_session_proposals",
        { session_id: sessionId },
        z.array(TaskProposalResponseSchema)
      );
      return raw.map(transformProposal);
    },

    /**
     * Update a proposal
     * @param proposalId The proposal ID
     * @param input Partial update data
     * @returns The updated proposal
     */
    update: async (proposalId: string, input: UpdateProposalInput): Promise<TaskProposalResponse> => {
      const raw = await typedInvoke(
        "update_task_proposal",
        {
          id: proposalId,
          input: {
            title: input.title,
            description: input.description,
            category: input.category,
            steps: input.steps,
            acceptance_criteria: input.acceptanceCriteria,
            user_priority: input.userPriority,
            complexity: input.complexity,
          },
        },
        TaskProposalResponseSchema
      );
      return transformProposal(raw);
    },

    /**
     * Delete a proposal
     * @param proposalId The proposal ID
     */
    delete: async (proposalId: string): Promise<void> => {
      await invoke("delete_task_proposal", { id: proposalId });
    },

    /**
     * Toggle proposal selection state
     * @param proposalId The proposal ID
     * @returns The new selection state
     */
    toggleSelection: async (proposalId: string): Promise<boolean> => {
      return typedInvoke(
        "toggle_proposal_selection",
        { id: proposalId },
        z.boolean()
      );
    },

    /**
     * Set proposal selection state
     * @param proposalId The proposal ID
     * @param selected Whether the proposal should be selected
     */
    setSelection: async (proposalId: string, selected: boolean): Promise<void> => {
      await invoke("set_proposal_selection", { id: proposalId, selected });
    },

    /**
     * Reorder proposals within a session
     * @param sessionId The session ID
     * @param proposalIds Array of proposal IDs in desired order
     */
    reorder: async (sessionId: string, proposalIds: string[]): Promise<void> => {
      await invoke("reorder_proposals", {
        session_id: sessionId,
        proposal_ids: proposalIds,
      });
    },

    /**
     * Assess priority for a single proposal
     * @param proposalId The proposal ID
     * @returns Priority assessment result
     */
    assessPriority: async (proposalId: string): Promise<PriorityAssessmentResponse> => {
      const raw = await typedInvoke(
        "assess_proposal_priority",
        { id: proposalId },
        PriorityAssessmentResponseSchema
      );
      return transformPriorityAssessment(raw);
    },

    /**
     * Assess priority for all proposals in a session
     * @param sessionId The session ID
     * @returns Array of priority assessments
     */
    assessAllPriorities: async (sessionId: string): Promise<PriorityAssessmentResponse[]> => {
      const raw = await typedInvoke(
        "assess_all_priorities",
        { session_id: sessionId },
        z.array(PriorityAssessmentResponseSchema)
      );
      return raw.map(transformPriorityAssessment);
    },
  },

  /**
   * Proposal dependency operations
   */
  dependencies: {
    /**
     * Add a dependency between proposals
     * @param proposalId The proposal that depends on another
     * @param dependsOnId The proposal that is depended on
     */
    add: async (proposalId: string, dependsOnId: string): Promise<void> => {
      await invoke("add_proposal_dependency", {
        proposal_id: proposalId,
        depends_on_id: dependsOnId,
      });
    },

    /**
     * Remove a dependency between proposals
     * @param proposalId The proposal that depends on another
     * @param dependsOnId The proposal that is depended on
     */
    remove: async (proposalId: string, dependsOnId: string): Promise<void> => {
      await invoke("remove_proposal_dependency", {
        proposal_id: proposalId,
        depends_on_id: dependsOnId,
      });
    },

    /**
     * Get proposals that this proposal depends on
     * @param proposalId The proposal ID
     * @returns Array of proposal IDs this depends on
     */
    getDependencies: async (proposalId: string): Promise<string[]> => {
      return typedInvoke(
        "get_proposal_dependencies",
        { proposal_id: proposalId },
        z.array(z.string())
      );
    },

    /**
     * Get proposals that depend on this proposal
     * @param proposalId The proposal ID
     * @returns Array of proposal IDs that depend on this
     */
    getDependents: async (proposalId: string): Promise<string[]> => {
      return typedInvoke(
        "get_proposal_dependents",
        { proposal_id: proposalId },
        z.array(z.string())
      );
    },

    /**
     * Analyze dependencies and build graph for a session
     * @param sessionId The session ID
     * @returns Dependency graph with nodes, edges, and cycle info
     */
    analyze: async (sessionId: string): Promise<DependencyGraphResponse> => {
      const raw = await typedInvoke(
        "analyze_dependencies",
        { session_id: sessionId },
        DependencyGraphResponseSchema
      );
      return transformDependencyGraph(raw);
    },
  },

  /**
   * Apply proposals to Kanban operations
   */
  apply: {
    /**
     * Apply selected proposals to the Kanban board as tasks
     * @param input Apply options
     * @returns Result with created task IDs and warnings
     */
    toKanban: async (input: ApplyProposalsInput): Promise<ApplyProposalsResultResponse> => {
      const raw = await typedInvoke(
        "apply_proposals_to_kanban",
        {
          input: {
            session_id: input.sessionId,
            proposal_ids: input.proposalIds,
            target_column: input.targetColumn,
            preserve_dependencies: input.preserveDependencies,
          },
        },
        ApplyProposalsResultResponseSchema
      );
      return transformApplyResult(raw);
    },
  },

  /**
   * Task dependency operations (for applied tasks)
   */
  taskDependencies: {
    /**
     * Get tasks that block a given task (tasks it depends on)
     * @param taskId The task ID
     * @returns Array of blocking task IDs
     */
    getBlockers: async (taskId: string): Promise<string[]> => {
      return typedInvoke(
        "get_task_blockers",
        { task_id: taskId },
        z.array(z.string())
      );
    },

    /**
     * Get tasks that are blocked by a given task
     * @param taskId The task ID
     * @returns Array of blocked task IDs
     */
    getBlocked: async (taskId: string): Promise<string[]> => {
      return typedInvoke(
        "get_blocked_tasks",
        { task_id: taskId },
        z.array(z.string())
      );
    },
  },
} as const;
