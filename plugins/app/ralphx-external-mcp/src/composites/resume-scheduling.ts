/**
 * Composite: resume_scheduling
 *
 * Resumes a failed v1_accept_plan_and_schedule from its last successful step.
 * Looks up the current state and retries the failed step.
 * Phase 5 implementation.
 */

import { getBackendClient, BackendError } from "../backend-client.js";
import type { ApiKeyContext } from "../types.js";

export interface ResumeSchedulingInput {
  sessionId: string;
}

export interface ResumeSchedulingResult {
  success: boolean;
  taskIds: string[];
  message: string;
}

interface ProposalListBody {
  proposals?: Array<{ id: string; [key: string]: unknown }>;
}

interface ApplyProposalsBody {
  task_ids?: string[];
  [key: string]: unknown;
}

/**
 * Resume a failed accept_plan_and_schedule by:
 * 1. Loading session proposals to determine what's already done
 * 2. Re-calling apply_proposals if not yet completed
 *
 * This is idempotent — apply_proposals is safe to call multiple times.
 */
export async function resumeScheduling(
  input: ResumeSchedulingInput,
  context: ApiKeyContext
): Promise<ResumeSchedulingResult> {
  // Step 1: Load proposals to check session state
  let proposalIds: string[];
  try {
    const listResp = await getBackendClient().get<ProposalListBody>(
      `/api/list_session_proposals/${encodeURIComponent(input.sessionId)}`,
      context
    );
    const proposals = listResp.body.proposals ?? [];
    proposalIds = proposals.map((p) => p.id);
  } catch (err) {
    const message = err instanceof BackendError
      ? `Backend error loading session: ${err.message}`
      : `Failed to load session: ${String(err)}`;
    return { success: false, taskIds: [], message };
  }

  if (proposalIds.length === 0) {
    return {
      success: true,
      taskIds: [],
      message: "No proposals to schedule — session is already complete or has no proposals.",
    };
  }

  // Step 2: Re-apply proposals (idempotent — backend handles already-applied proposals)
  try {
    const applyResp = await getBackendClient().post<ApplyProposalsBody>(
      "/api/external/apply_proposals",
      context,
      {
        session_id: input.sessionId,
        proposal_ids: proposalIds,
      }
    );

    if (applyResp.status < 200 || applyResp.status >= 300) {
      return {
        success: false,
        taskIds: [],
        message: `apply_proposals returned HTTP ${applyResp.status}`,
      };
    }

    const taskIds = applyResp.body.task_ids ?? [];
    return {
      success: true,
      taskIds,
      message: `Scheduling resumed successfully. ${taskIds.length} task(s) scheduled.`,
    };
  } catch (err) {
    const message = err instanceof BackendError
      ? `Backend error during scheduling: ${err.message}`
      : `Failed to apply proposals: ${String(err)}`;
    return { success: false, taskIds: [], message };
  }
}
