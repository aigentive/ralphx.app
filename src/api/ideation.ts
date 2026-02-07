// Tauri invoke wrappers for ideation system with type safety using Zod schemas

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import { IdeationSettingsResponseSchema } from "../types/ideation-config";
import type { IdeationSettings } from "../types/ideation-config";
import {
  IdeationSessionResponseSchema,
  TaskProposalResponseSchema,
  SessionWithDataResponseSchema,
  PriorityAssessmentResponseSchema,
  DependencyGraphResponseSchema,
  ApplyProposalsResultResponseSchema,
} from "./ideation.schemas";
import {
  transformSession,
  transformProposal,
  transformSessionWithData,
  transformPriorityAssessment,
  transformDependencyGraph,
  transformApplyResult,
  transformIdeationSettings,
} from "./ideation.transforms";
export { toTaskProposal } from "./ideation.transforms";
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
} from "./ideation.types";

// Re-export types for convenience
export type {
  IdeationSessionResponse,
  TaskProposalResponse,
  ChatMessageResponse,
  SessionWithDataResponse,
  PriorityAssessmentResponse,
  DependencyGraphNodeResponse,
  DependencyGraphEdgeResponse,
  DependencyGraphResponse,
  ApplyProposalsResultResponse,
  CreateProposalInput,
  UpdateProposalInput,
  ApplyProposalsInput,
} from "./ideation.types";


// ============================================================================
// Typed Invoke Helper
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
     * @param seedTaskId Optional draft task ID to seed the session
     * @returns The created session
     */
    create: async (projectId: string, title?: string, seedTaskId?: string): Promise<IdeationSessionResponse> => {
      const raw = await typedInvoke(
        "create_ideation_session",
        { input: { project_id: projectId, title, seed_task_id: seedTaskId } },
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
        { projectId },  // camelCase - Tauri auto-converts to snake_case for Rust
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

    /**
     * Reopen an accepted/archived session back to Active status
     * Deletes all tasks, cleans up git resources, clears proposal task links
     * @param sessionId The session ID
     */
    reopen: async (sessionId: string): Promise<void> => {
      await invoke("reopen_ideation_session", { id: sessionId });
    },

    /**
     * Update a session's title
     * @param sessionId The session ID
     * @param title The new title (or null to clear)
     * @returns The updated session
     */
    updateTitle: async (sessionId: string, title: string | null): Promise<IdeationSessionResponse> => {
      const raw = await typedInvoke(
        "update_ideation_session_title",
        { id: sessionId, title },
        IdeationSessionResponseSchema
      );
      return transformSession(raw);
    },

    /**
     * Spawn a session-namer agent to auto-generate a title from the first message
     * Runs in background and returns immediately (fire-and-forget)
     * @param sessionId The session ID
     * @param firstMessage The user's first message in the session
     */
    spawnSessionNamer: async (sessionId: string, firstMessage: string): Promise<void> => {
      await invoke("spawn_session_namer", { sessionId, firstMessage });
    },

    /**
     * Spawn a dependency-suggester agent to analyze proposals and suggest dependencies
     * Runs in background and returns immediately (fire-and-forget)
     * Requires at least 2 proposals in the session.
     * @param sessionId The session ID
     */
    spawnDependencySuggester: async (sessionId: string): Promise<void> => {
      await invoke("spawn_dependency_suggester", { sessionId });
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
        { sessionId },  // camelCase - Tauri auto-converts to snake_case for Rust
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
     * Reorder proposals within a session
     * @param sessionId The session ID
     * @param proposalIds Array of proposal IDs in desired order
     */
    reorder: async (sessionId: string, proposalIds: string[]): Promise<void> => {
      await invoke("reorder_proposals", {
        sessionId,
        proposalIds,
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
        { sessionId },  // camelCase - Tauri auto-converts to snake_case for Rust
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
        proposalId,
        dependsOnId,
      });
    },

    /**
     * Remove a dependency between proposals
     * @param proposalId The proposal that depends on another
     * @param dependsOnId The proposal that is depended on
     */
    remove: async (proposalId: string, dependsOnId: string): Promise<void> => {
      await invoke("remove_proposal_dependency", {
        proposalId,
        dependsOnId,
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
        { proposalId },
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
        { proposalId },
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
        { sessionId },
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
     * Apply proposals to the Kanban board as tasks
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
            ...(input.useFeatureBranch !== undefined && {
              use_feature_branch: input.useFeatureBranch,
            }),
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
        { taskId },
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
        { taskId },
        z.array(z.string())
      );
    },
  },

  /**
   * Ideation settings operations
   */
  settings: {
    /**
     * Get current ideation settings
     * @returns Current ideation settings
     */
    get: async (): Promise<IdeationSettings> => {
      const raw = await typedInvoke(
        "get_ideation_settings",
        {},
        IdeationSettingsResponseSchema
      );
      return transformIdeationSettings(raw);
    },

    /**
     * Update ideation settings
     * @param settings New settings to apply
     * @returns Updated settings
     */
    update: async (settings: IdeationSettings): Promise<IdeationSettings> => {
      const raw = await typedInvoke(
        "update_ideation_settings",
        {
          settings: {
            plan_mode: settings.planMode,
            require_plan_approval: settings.requirePlanApproval,
            suggest_plans_for_complex: settings.suggestPlansForComplex,
            auto_link_proposals: settings.autoLinkProposals,
          },
        },
        IdeationSettingsResponseSchema
      );
      return transformIdeationSettings(raw);
    },
  },
} as const;
