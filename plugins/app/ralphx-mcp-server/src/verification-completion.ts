import {
  aggregateVerificationGaps,
  parseTypedVerificationFinding,
  type VerificationFindingSummary,
  type VerificationRoundDelegateInput,
} from "./verification-round-assessment.js";

export type VerificationTerminalBody = {
  status: string;
  generation: number;
  round?: number;
  convergence_reason?: string;
  [key: string]: unknown;
};

export type VerificationSettlementResult = {
  classification: "complete" | "pending" | "infra_failure";
  missing_required_critics: string[];
  verification_findings?: VerificationFindingSummary[];
  recommended_next_action?: string;
  [key: string]: unknown;
};

const TERMINAL_NEEDS_REVISION_REASONS = new Set([
  "max_rounds",
  "critic_parse_failure",
  "user_stopped",
  "user_skipped",
  "user_reverted",
  "escalated_to_parent",
]);

export async function completePlanVerificationWithSettlement(
  deps: {
    sessionId: string;
    body: VerificationTerminalBody;
    requiredDelegates: VerificationRoundDelegateInput[];
    createdAfter?: string;
    rescueBudgetExhausted: boolean;
    includeFullContent: boolean;
    includeMessages: boolean;
    messageLimit: number;
    maxWaitMs: number;
    pollIntervalMs: number;
    awaitVerificationRoundSettlement: (args: {
      session_id: string;
      delegates: VerificationRoundDelegateInput[];
      created_after?: string;
      rescue_budget_exhausted?: boolean;
      include_full_content?: boolean;
      include_messages?: boolean;
      message_limit?: number;
      max_wait_ms?: number;
      poll_interval_ms?: number;
    }) => Promise<VerificationSettlementResult>;
    callInfraFailure: (args: {
      generation: number;
      convergence_reason?: string;
      round?: number;
    }) => Promise<Record<string, unknown>>;
    callCompletion: (body: Record<string, unknown>) => Promise<unknown>;
  }
): Promise<Record<string, unknown> | unknown> {
  const settledRound = await deps.awaitVerificationRoundSettlement({
    session_id: deps.sessionId,
    delegates: deps.requiredDelegates,
    created_after: deps.createdAfter,
    rescue_budget_exhausted: deps.rescueBudgetExhausted,
    include_full_content: deps.includeFullContent,
    include_messages: deps.includeMessages,
    message_limit: deps.messageLimit,
    max_wait_ms: deps.maxWaitMs,
    poll_interval_ms: deps.pollIntervalMs,
  });

  if (settledRound.classification === "pending") {
    throw new Error(
      `Required verification delegates are still pending for: ${settledRound.missing_required_critics.join(", ")}. Wait for settlement before terminal completion.`
    );
  }

  if (deps.body.status === "verified" && settledRound.classification !== "complete") {
    throw new Error(
      `Cannot complete verification as verified while required delegate coverage is ${settledRound.classification}.`
    );
  }

  let completionBody: Record<string, unknown> = {
    ...deps.body,
  };

  if (settledRound.classification === "complete") {
    const verificationFindings = Array.isArray(settledRound.verification_findings)
      ? settledRound.verification_findings
      : [];
    const nextAction = typeof settledRound.recommended_next_action === "string"
      ? settledRound.recommended_next_action
      : undefined;
    if (
      deps.body.status === "needs_revision" &&
      (
        !deps.body.convergence_reason
        || !TERMINAL_NEEDS_REVISION_REASONS.has(deps.body.convergence_reason)
        || nextAction === "continue_round_analysis"
      )
    ) {
      throw new Error(
        "Cannot complete verification as needs_revision while backend settlement still requires bounded revision. Revise the plan and continue the active verification loop instead of forcing terminal cleanup."
      );
    }
    const requiredLabels = Array.from(
      new Set(
        deps.requiredDelegates
          .map((delegate) => delegate.label?.trim().toLowerCase())
          .filter((label): label is string => Boolean(label))
      )
    );
    const parsedRequiredFindings = requiredLabels.map((label) =>
      parseTypedVerificationFinding({
        label,
        finding: verificationFindings.find(
          (entry) => entry.critic.trim().toLowerCase() === label
        ),
      })
    );
    const unusableRequired = parsedRequiredFindings.filter((finding) => !finding.usable);
    if (unusableRequired.length > 0) {
      throw new Error(
        `Required verification findings were published but unusable: ${unusableRequired
          .map((finding) => finding.label)
          .join(", ")}.`
      );
    }
    const { merged_gaps, gap_counts } = aggregateVerificationGaps(parsedRequiredFindings);
    if (
      deps.body.status === "verified" &&
      (gap_counts.critical > 0 || gap_counts.high > 0 || gap_counts.medium > 0)
    ) {
      throw new Error(
        "Cannot complete verification as verified while required findings still contain blocking gaps."
      );
    }
    completionBody = {
      ...completionBody,
      gaps: merged_gaps,
    };
  }

  if (settledRound.classification === "infra_failure") {
    const infraFailure = await deps.callInfraFailure({
      generation: deps.body.generation,
      convergence_reason: deps.body.convergence_reason,
      round: deps.body.round,
    });
    return {
      ...infraFailure,
      settlement: settledRound,
    };
  }

  const completion = await deps.callCompletion({
    ...completionBody,
    in_progress: false,
  });
  return {
    ...(completion as Record<string, unknown>),
    settlement: settledRound,
  };
}
