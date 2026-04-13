import {
  assessVerificationRound,
  type VerificationFindingSummary,
  type VerificationRoundDelegateInput,
  type VerificationRoundDelegateSnapshot,
} from "./verification-round-assessment.js";
import {
  runVerificationEnrichmentPass,
  runVerificationRoundPass,
  type RequiredCriticRoundResult,
  type VerificationPlanSnapshot,
} from "./verification-orchestration.js";
import { completePlanVerificationWithSettlement } from "./verification-completion.js";

export type TeamArtifactSummary = {
  id: string;
  name: string;
  artifact_type: string;
  version: number;
  content_preview: string;
  created_at: string;
  author_teammate?: string | null;
};

type VerificationFindingMatch = {
  critic: string;
  found: boolean;
  total_matches: number;
  finding?: VerificationFindingSummary;
};

type DelegationJobSnapshot = VerificationRoundDelegateSnapshot & {
  delegated_status?: unknown;
};

export type VerificationSettlementArgs = {
  session_id: string;
  delegates: VerificationRoundDelegateInput[];
  created_after?: string;
  rescue_budget_exhausted?: boolean;
  include_full_content?: boolean;
  include_messages?: boolean;
  message_limit?: number;
  max_wait_ms?: number;
  poll_interval_ms?: number;
};

export type VerificationAssessmentArgs = {
  session_id: string;
  delegates: VerificationRoundDelegateInput[];
  created_after?: string;
  rescue_budget_exhausted?: boolean;
  include_messages?: boolean;
  message_limit?: number;
};

type ManagedVerificationDelegate = VerificationRoundDelegateInput & {
  agent_name: string;
  delegated_session_id?: string;
};

type AwaitVerificationRoundSettlementResult = {
  session_id: string;
  created_after: string | null;
  rescue_budget_exhausted: boolean;
  settled: boolean;
  timed_out: boolean;
  polls_performed: number;
  max_wait_ms: number;
  poll_interval_ms: number;
  verification_findings: VerificationFindingSummary[];
  classification: "complete" | "pending" | "infra_failure";
  recommended_next_action:
    | "continue_round_analysis"
    | "perform_single_rescue_or_wait"
    | "complete_verification_with_infra_failure";
  summary: string;
  missing_required_prefixes: string[];
  delegate_assessments: ReturnType<typeof assessVerificationRound>["delegate_assessments"];
  artifacts_by_prefix: ReturnType<typeof assessVerificationRound>["artifacts_by_prefix"];
  delegate_snapshots: VerificationRoundDelegateSnapshot[];
};

type VerificationRuntimeDeps = {
  callTauri: (endpoint: string, payload: Record<string, unknown>) => Promise<unknown>;
  callTauriGet: (endpoint: string) => Promise<unknown>;
  agentType: string;
  contextType?: string;
  contextId?: string;
};

export function createVerificationRuntime(deps: VerificationRuntimeDeps) {
  const { callTauri, callTauriGet, agentType, contextType, contextId } = deps;
  const VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS = 90 * 1000;
  const VERIFICATION_REQUIRED_WAIT_DEFAULT_MS = VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS;
  const VERIFICATION_OPTIONAL_WAIT_DEFAULT_MS = 15 * 1000;
  const VERIFICATION_ENRICHMENT_WAIT_DEFAULT_MS = 15 * 1000;
  const VERIFICATION_RESCUE_WAIT_SLICE_MS = 15 * 1000;

  function normalizeMessageLimit(messageLimit?: number): number {
    return Math.min(Math.max(messageLimit ?? 5, 1), 50);
  }

  function normalizeMaxWaitMs(
    maxWaitMs: number | undefined,
    fallback: number,
    cap = VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS
  ): number {
    return Math.min(
      Math.max(maxWaitMs ?? fallback, 0),
      cap
    );
  }

  function normalizePollIntervalMs(pollIntervalMs: number | undefined, fallback: number): number {
    return Math.max(pollIntervalMs ?? fallback, 100);
  }

  function isRunningLikeStatus(status?: string | null): boolean {
    return (
      status === "running" ||
      status === "queued" ||
      status === "likely_generating" ||
      status === "likely_waiting"
    );
  }
  function selectLatestArtifactsByPrefix(
    artifacts: TeamArtifactSummary[],
    prefixes: string[],
    createdAfter?: string
  ): Array<{
    prefix: string;
    found: boolean;
    total_matches: number;
    artifact?: TeamArtifactSummary;
  }> {
    const createdAfterMs =
      typeof createdAfter === "string" && createdAfter.length > 0
        ? Date.parse(createdAfter)
        : Number.NaN;
    const hasThreshold = Number.isFinite(createdAfterMs);
  
    return prefixes.map((prefix) => {
      const matches = artifacts
        .filter((artifact) => artifact.name.startsWith(prefix))
        .filter((artifact) => {
          if (!hasThreshold) return true;
          const createdAtMs = Date.parse(artifact.created_at);
          return Number.isFinite(createdAtMs) && createdAtMs >= createdAfterMs;
        })
        .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at));
  
      const latest = matches[0];
      return latest
        ? {
            prefix,
            found: true,
            total_matches: matches.length,
            artifact: latest,
          }
        : {
            prefix,
            found: false,
            total_matches: 0,
          };
    });
  }
  
  async function sleep(ms: number): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, ms));
  }
  
  async function loadVerificationArtifactsByPrefix(args: {
    sessionId: string;
    prefixes: string[];
    createdAfter?: string;
    includeFullContent: boolean;
  }) {
    const teamArtifacts = await callTauriGet(`team/artifacts/${args.sessionId}`) as {
      artifacts: TeamArtifactSummary[];
      count: number;
    };
    const matches = selectLatestArtifactsByPrefix(
      teamArtifacts.artifacts ?? [],
      args.prefixes,
      args.createdAfter
    );
    return await Promise.all(
      matches.map(async (match) => {
        if (!match.artifact || !args.includeFullContent) {
          return match;
        }
        const fullArtifact = await callTauriGet(`artifact/${match.artifact.id}`) as {
          content?: string;
        };
        return {
          ...match,
          artifact: {
            ...match.artifact,
            content: fullArtifact.content ?? "",
          },
        };
      })
    );
  }
  
  function selectLatestVerificationFindingsByCritic(
    findings: VerificationFindingSummary[],
    critics: string[],
    createdAfter?: string,
    round?: number
  ): VerificationFindingMatch[] {
    const createdAfterMs =
      typeof createdAfter === "string" && createdAfter.length > 0
        ? Date.parse(createdAfter)
        : Number.NaN;
    const hasThreshold = Number.isFinite(createdAfterMs);
  
    return critics.map((critic) => {
      const normalizedCritic = critic.trim().toLowerCase();
      const matches = findings
        .filter((finding) => finding.critic.trim().toLowerCase() === normalizedCritic)
        .filter((finding) => (typeof round === "number" ? finding.round === round : true))
        .filter((finding) => {
          if (!hasThreshold) {
            return true;
          }
          const createdAtMs = Date.parse(finding.created_at);
          return Number.isFinite(createdAtMs) && createdAtMs >= createdAfterMs;
        })
        .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at));
  
      const latest = matches[0];
      return latest
        ? {
            critic: normalizedCritic,
            found: true,
            total_matches: matches.length,
            finding: latest,
          }
        : {
            critic: normalizedCritic,
            found: false,
            total_matches: 0,
          };
    });
  }
  
  async function loadVerificationFindingsByCritic(args: {
    sessionId: string;
    critics: string[];
    round?: number;
    createdAfter?: string;
  }) {
    const searchParams = new URLSearchParams();
    if (typeof args.round === "number") {
      searchParams.set("round", String(args.round));
    }
    if (typeof args.createdAfter === "string" && args.createdAfter.length > 0) {
      searchParams.set("created_after", args.createdAfter);
    }
  
    const query = searchParams.toString();
    const response = await callTauriGet(
      `team/verification-findings/${args.sessionId}${query.length > 0 ? `?${query}` : ""}`
    ) as {
      findings: VerificationFindingSummary[];
      count: number;
    };
  
    return selectLatestVerificationFindingsByCritic(
      response.findings ?? [],
      args.critics,
      args.createdAfter,
      args.round
    );
  }
  
  async function loadVerificationDelegateSnapshots(args: {
    delegates: VerificationRoundDelegateInput[];
    includeMessages: boolean;
    messageLimit: number;
  }) {
    return await Promise.all(
      args.delegates.map(async (delegate) => {
        try {
          return await callTauri("coordination/delegate/wait", {
            job_id: delegate.job_id,
            include_delegated_status: true,
            include_messages: args.includeMessages,
            message_limit: args.messageLimit,
          }) as DelegationJobSnapshot;
        } catch (error) {
          const errorMessage =
            error instanceof Error ? error.message : String(error);
          return {
            job_id: delegate.job_id,
            status: "failed",
            error: errorMessage,
          } satisfies DelegationJobSnapshot;
        }
      })
    );
  }
  
  type RequiredCriticDefinition = {
    agent_name:
      | "ralphx:ralphx-plan-critic-completeness"
      | "ralphx:ralphx-plan-critic-implementation-feasibility";
    critic: "completeness" | "feasibility";
    artifact_prefix: "Completeness: " | "Feasibility: ";
    label: "completeness" | "feasibility";
    initial_prompt: (sessionId: string, round: number) => string;
    rescue_prompt: (sessionId: string, round: number) => string;
  };
  
  const REQUIRED_VERIFICATION_CRITICS: RequiredCriticDefinition[] = [
    {
      agent_name: "ralphx:ralphx-plan-critic-completeness",
      critic: "completeness",
      artifact_prefix: "Completeness: ",
      label: "completeness",
      initial_prompt: (sessionId, round) =>
        `SESSION_ID: ${sessionId}\nROUND: ${round}\nRead the current plan, stay bounded to the Affected Files plus at most one adjacent integration point per file family, then publish exactly one completeness verification finding with publish_verification_finding. Use critic='completeness'. If analysis is incomplete, publish status='partial' immediately instead of continuing to explore.`,
      rescue_prompt: (sessionId, round) =>
        `SESSION_ID: ${sessionId}\nROUND: ${round}\nCompleteness rescue pass. Publish the completeness verification finding now with publish_verification_finding. If analysis is partial, publish status='partial' instead of exploring further.`,
    },
    {
      agent_name: "ralphx:ralphx-plan-critic-implementation-feasibility",
      critic: "feasibility",
      artifact_prefix: "Feasibility: ",
      label: "feasibility",
      initial_prompt: (sessionId, round) =>
        `SESSION_ID: ${sessionId}\nROUND: ${round}\nRead the current plan, stay bounded to the Affected Files plus at most one adjacent integration point per file family, then publish exactly one feasibility verification finding with publish_verification_finding. Use critic='feasibility'. If analysis is incomplete, publish status='partial' immediately instead of continuing to explore.`,
      rescue_prompt: (sessionId, round) =>
        `SESSION_ID: ${sessionId}\nROUND: ${round}\nFeasibility rescue pass. Publish the feasibility verification finding now with publish_verification_finding. If analysis is partial, publish status='partial' instead of exploring further.`,
    },
  ];
  
  async function loadVerificationPlanSnapshot(sessionId: string): Promise<VerificationPlanSnapshot> {
    const planData = await callTauriGet(`get_session_plan/${sessionId}`) as {
      id?: string;
      artifact_id?: string;
      content?: string;
      project_working_directory?: string | null;
    };
    return {
      artifact_id:
        typeof planData.artifact_id === "string"
          ? planData.artifact_id
          : typeof planData.id === "string"
            ? planData.id
            : undefined,
      content: typeof planData.content === "string" ? planData.content : "",
      project_working_directory:
        typeof planData.project_working_directory === "string"
          ? planData.project_working_directory
          : null,
    };
  }
  
  async function awaitOptionalVerificationDelegates(args: {
    delegates: ManagedVerificationDelegate[];
    sessionId: string;
    createdAfter: string;
    prefixes: string[];
    includeFullContent: boolean;
    includeMessages: boolean;
    messageLimit: number;
    maxWaitMs: number;
    pollIntervalMs: number;
  }) {
    const deadline =
      Date.now() +
      Math.min(Math.max(args.maxWaitMs, 0), VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS);
    let pollsPerformed = 0;
  
    while (true) {
      pollsPerformed += 1;
      const artifactsByPrefix = await loadVerificationArtifactsByPrefix({
        sessionId: args.sessionId,
        prefixes: args.prefixes,
        createdAfter: args.createdAfter,
        includeFullContent: args.includeFullContent,
      });
      const delegateSnapshots = await loadVerificationDelegateSnapshots({
        delegates: args.delegates,
        includeMessages: args.includeMessages,
        messageLimit: args.messageLimit,
      });
  
      const allSettled = args.delegates.every((delegate) => {
        const artifact = artifactsByPrefix.find(
          (entry) => entry.prefix === delegate.artifact_prefix
        );
        if (artifact?.found === true) {
          return true;
        }
        const snapshot = delegateSnapshots.find(
          (entry) => entry.job_id === delegate.job_id
        );
        const statuses = [
          snapshot?.status,
          snapshot?.delegated_status?.latest_run?.status ?? undefined,
          snapshot?.delegated_status?.agent_state?.estimated_status ?? undefined,
        ];
        return statuses.some(
          (status) =>
            status === "completed" || status === "failed" || status === "cancelled"
        );
      });
  
      if (allSettled || Date.now() >= deadline) {
        return {
          created_after: args.createdAfter,
          polls_performed: pollsPerformed,
          timed_out: !allSettled,
          delegates: args.delegates.map(
            ({ job_id, artifact_prefix, label, required }) => ({
              job_id,
              artifact_prefix,
              label,
              required,
            })
          ),
          artifacts_by_prefix: artifactsByPrefix,
          delegate_snapshots: delegateSnapshots,
        };
      }
  
      await sleep(args.pollIntervalMs);
    }
  }
  
  async function startManagedVerificationDelegate(args: {
    agentName: string;
    parentSessionId: string;
    prompt: string;
    delegatedSessionId?: string;
  }) {
    return await callTauri("coordination/delegate/start", {
      agent_name: args.agentName,
      parent_session_id: args.parentSessionId,
      prompt: args.prompt,
      delegated_session_id: args.delegatedSessionId,
      caller_agent_name: agentType,
      caller_context_type: contextType,
      caller_context_id: contextId,
    }) as {
      job_id: string;
      delegated_session_id?: string;
      agent_name: string;
      harness?: string;
      status?: string;
    };
  }
  
  function summarizeVerificationInfraFailure(args: {
    sessionId: string;
    round: number;
    createdAfter: string;
    delegates: ManagedVerificationDelegate[];
    summary: string;
    error?: string;
    rescueDispatched?: boolean;
    rescueDelegates?: ManagedVerificationDelegate[];
  }): RequiredCriticRoundResult {
    const rescueDelegates = args.rescueDelegates ?? [];
    return {
      session_id: args.sessionId,
      round: args.round,
      created_after: args.createdAfter,
      rescue_dispatched: args.rescueDispatched === true,
      required_delegates: args.delegates.map(({ job_id, artifact_prefix, label, required }) => ({
        job_id,
        artifact_prefix,
        label,
        required,
      })),
      rescue_delegates: rescueDelegates.map(({ job_id, artifact_prefix, label, required }) => ({
        job_id,
        artifact_prefix,
        label,
        required,
      })),
      settlement: {
        session_id: args.sessionId,
        created_after: args.createdAfter,
        rescue_budget_exhausted: args.rescueDispatched === true,
        settled: true,
        timed_out: false,
        polls_performed: 0,
        classification: "infra_failure",
        recommended_next_action: "complete_verification_with_infra_failure",
        summary: args.summary,
        missing_required_prefixes: REQUIRED_VERIFICATION_CRITICS
          .filter(
            (critic) =>
              !args.delegates.some((delegate) => delegate.artifact_prefix === critic.artifact_prefix)
          )
          .map((critic) => critic.artifact_prefix),
        delegate_assessments: [],
        artifacts_by_prefix: [],
        error: args.error ?? null,
      },
    };
  }
  
  async function runRequiredVerificationCriticRound(args: {
    sessionId: string;
    round: number;
    includeFullContent: boolean;
    includeMessages: boolean;
    messageLimit: number;
    maxWaitMs: number;
    pollIntervalMs: number;
  }): Promise<RequiredCriticRoundResult> {
    const dispatchStartedAt = Date.now();
    const createdAfter = new Date(dispatchStartedAt - 5000).toISOString();
    const totalWaitBudgetMs = normalizeMaxWaitMs(
      args.maxWaitMs,
      VERIFICATION_REQUIRED_WAIT_DEFAULT_MS
    );
    const rescueWaitBudgetMs =
      totalWaitBudgetMs > VERIFICATION_RESCUE_WAIT_SLICE_MS
        ? VERIFICATION_RESCUE_WAIT_SLICE_MS
        : Math.max(args.pollIntervalMs, Math.floor(totalWaitBudgetMs / 2));
    const initialWaitBudgetMs = Math.max(
      totalWaitBudgetMs - rescueWaitBudgetMs,
      args.pollIntervalMs
    );
  
    const initialLaunches = await Promise.all(
      REQUIRED_VERIFICATION_CRITICS.map(async (critic) => {
        try {
          const launched = await startManagedVerificationDelegate({
            agentName: critic.agent_name,
            parentSessionId: args.sessionId,
            prompt: critic.initial_prompt(args.sessionId, args.round),
          });
          return {
            ok: true as const,
            critic,
            delegate: {
              job_id: launched.job_id,
              delegated_session_id: launched.delegated_session_id,
              agent_name: critic.agent_name,
              artifact_prefix: critic.artifact_prefix,
              label: critic.label,
              required: true,
            } satisfies ManagedVerificationDelegate,
          };
        } catch (error) {
          return {
            ok: false as const,
            critic,
            error: error instanceof Error ? error.message : String(error),
          };
        }
      })
    );
  
    const initialDelegates = initialLaunches
      .filter((launch) => launch.ok)
      .map((launch) => launch.delegate);
    const initialLaunchFailures = initialLaunches.filter((launch) => !launch.ok);
  
    if (initialLaunchFailures.length > 0) {
      return summarizeVerificationInfraFailure({
        sessionId: args.sessionId,
        round: args.round,
        createdAfter,
        delegates: initialDelegates,
        summary:
          "Required critic dispatch failed before the verification round could settle.",
        error: initialLaunchFailures
          .map((failure) => `${failure.critic.label}: ${failure.error}`)
          .join("; "),
      });
    }
  
    const initialDelegateInputs = initialDelegates.map(
      ({ job_id, artifact_prefix, label, required }) => ({
        job_id,
        artifact_prefix,
        label,
        required,
      })
    );
  
    const firstSettlement = await awaitVerificationRoundSettlement({
      session_id: args.sessionId,
      delegates: initialDelegateInputs,
      created_after: createdAfter,
      rescue_budget_exhausted: false,
      include_full_content: args.includeFullContent,
      include_messages: args.includeMessages,
      message_limit: args.messageLimit,
      max_wait_ms: initialWaitBudgetMs,
      poll_interval_ms: args.pollIntervalMs,
    });
  
    if (firstSettlement.classification !== "pending") {
      return {
        session_id: args.sessionId,
        round: args.round,
        created_after: createdAfter,
        rescue_dispatched: false,
        required_delegates: initialDelegateInputs,
        rescue_delegates: [],
        settlement: firstSettlement,
      };
    }
  
    const rescueTargets = initialDelegates.filter((delegate) => {
      if (!firstSettlement.missing_required_prefixes.includes(delegate.artifact_prefix)) {
        return false;
      }
      const snapshot = firstSettlement.delegate_snapshots.find(
        (entry) => entry.job_id === delegate.job_id
      );
      const statuses = [
        snapshot?.status,
        snapshot?.delegated_status?.latest_run?.status ?? null,
        snapshot?.delegated_status?.agent_state?.estimated_status ?? null,
      ];
      return !statuses.some((status) => isRunningLikeStatus(status));
    });

    if (rescueTargets.length === 0) {
      return {
        session_id: args.sessionId,
        round: args.round,
        created_after: createdAfter,
        rescue_dispatched: false,
        required_delegates: initialDelegateInputs,
        rescue_delegates: [],
        settlement: firstSettlement,
      };
    }
  
    const rescueLaunches = await Promise.all(
      rescueTargets.map(async (target) => {
        const critic = REQUIRED_VERIFICATION_CRITICS.find(
          (entry) => entry.artifact_prefix === target.artifact_prefix
        );
        if (!critic) {
          return {
            ok: false as const,
            delegate: target,
            error: `Unknown required critic prefix ${target.artifact_prefix}`,
          };
        }
        try {
          const launched = await startManagedVerificationDelegate({
            agentName: critic.agent_name,
            parentSessionId: args.sessionId,
            delegatedSessionId: target.delegated_session_id,
            prompt: critic.rescue_prompt(args.sessionId, args.round),
          });
          return {
            ok: true as const,
            target,
            delegate: {
              job_id: launched.job_id,
              delegated_session_id:
                launched.delegated_session_id ?? target.delegated_session_id,
              agent_name: critic.agent_name,
              artifact_prefix: critic.artifact_prefix,
              label: critic.label,
              required: true,
            } satisfies ManagedVerificationDelegate,
          };
        } catch (error) {
          return {
            ok: false as const,
            delegate: target,
            error: error instanceof Error ? error.message : String(error),
          };
        }
      })
    );
  
    const rescueFailures = rescueLaunches.filter((launch) => !launch.ok);
    const successfulRescues = rescueLaunches
      .filter((launch) => launch.ok)
      .map((launch) => launch.delegate);
  
    if (rescueFailures.length > 0) {
      return summarizeVerificationInfraFailure({
        sessionId: args.sessionId,
        round: args.round,
        createdAfter,
        delegates: initialDelegates,
        rescueDispatched: true,
        rescueDelegates: successfulRescues,
        summary:
          "A required critic rescue dispatch failed, so the verification round cannot be trusted as plan feedback.",
        error: rescueFailures
          .map((failure) => `${failure.delegate.label ?? failure.delegate.artifact_prefix}: ${failure.error}`)
          .join("; "),
      });
    }
  
    const finalDelegates = initialDelegates.map((delegate) => {
      const replacement = successfulRescues.find(
        (rescue) => rescue.artifact_prefix === delegate.artifact_prefix
      );
      return replacement ?? delegate;
    });
    const finalDelegateInputs = finalDelegates.map(
      ({ job_id, artifact_prefix, label, required }) => ({
        job_id,
        artifact_prefix,
        label,
        required,
      })
    );
  
    const finalSettlement = await awaitVerificationRoundSettlement({
      session_id: args.sessionId,
      delegates: finalDelegateInputs,
      created_after: createdAfter,
      rescue_budget_exhausted: true,
      include_full_content: args.includeFullContent,
      include_messages: args.includeMessages,
      message_limit: args.messageLimit,
      max_wait_ms: rescueWaitBudgetMs,
      poll_interval_ms: args.pollIntervalMs,
    });
  
    return {
      session_id: args.sessionId,
      round: args.round,
      created_after: createdAfter,
      rescue_dispatched: true,
      required_delegates: finalDelegateInputs,
      rescue_delegates: successfulRescues.map(
        ({ job_id, artifact_prefix, label, required }) => ({
          job_id,
          artifact_prefix,
          label,
          required,
        })
      ),
      settlement: finalSettlement,
    };
  }
  
  async function awaitVerificationRoundSettlement(
    args: VerificationSettlementArgs
  ): Promise<AwaitVerificationRoundSettlementResult> {
    const includeMessages = args.include_messages !== false;
    const messageLimit = Math.min(Math.max(args.message_limit ?? 5, 1), 50);
    const rescueBudgetExhausted = args.rescue_budget_exhausted === true;
    const maxWaitMs = Math.min(
      Math.max(args.max_wait_ms ?? VERIFICATION_REQUIRED_WAIT_DEFAULT_MS, 0),
      VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS
    );
    const pollIntervalMs = Math.max(args.poll_interval_ms ?? 750, 100);
    const uniquePrefixes = Array.from(
      new Set(args.delegates.map((delegate) => delegate.artifact_prefix))
    );
  
    let pollsPerformed = 0;
    let timedOut = false;
    const deadline = Date.now() + maxWaitMs;
  
    while (true) {
      pollsPerformed += 1;
      const findingMatches = await loadVerificationFindingsByCritic({
        sessionId: args.session_id,
        critics: Array.from(
          new Set(
            args.delegates
              .map((delegate) => delegate.label?.trim().toLowerCase())
              .filter((label): label is string => Boolean(label))
          )
        ),
        createdAfter: args.created_after,
      });
      const findingByCritic = new Map(
        findingMatches.map((match) => [match.critic, match] as const)
      );
      const artifacts_by_prefix = uniquePrefixes.map((prefix) => {
        const delegate = args.delegates.find((entry) => entry.artifact_prefix === prefix);
        const critic = delegate?.label?.trim().toLowerCase();
        const findingMatch = critic ? findingByCritic.get(critic) : undefined;
        return findingMatch?.found && findingMatch.finding
          ? {
              prefix,
              found: true,
              total_matches: findingMatch.total_matches,
              artifact: {
                id: findingMatch.finding.artifact_id,
                name: findingMatch.finding.title,
                created_at: findingMatch.finding.created_at,
              },
            }
          : {
              prefix,
              found: false,
              total_matches: 0,
            };
      });
      const delegateSnapshots = await loadVerificationDelegateSnapshots({
        delegates: args.delegates,
        includeMessages,
        messageLimit,
      });
      const assessment = assessVerificationRound({
        delegates: args.delegates,
        artifactsByPrefix: artifacts_by_prefix,
        delegateSnapshots,
        rescueBudgetExhausted,
      });
      const settled = assessment.classification !== "pending";
  
      if (settled || Date.now() >= deadline) {
        timedOut = !settled && assessment.classification === "pending";
        const finalAssessment =
          timedOut && rescueBudgetExhausted
            ? {
                ...assessment,
                classification: "pending" as const,
                recommended_next_action: "perform_single_rescue_or_wait" as const,
                summary:
                  assessment.missing_required_prefixes.length > 0
                    ? `Required verification delegates are still running after the current bounded wait budget: ${assessment.missing_required_prefixes.join(", ")}.`
                    : "Required verification delegates are still running after the current bounded wait budget.",
              }
            : assessment;
        return {
          session_id: args.session_id,
          created_after: args.created_after ?? null,
          rescue_budget_exhausted: rescueBudgetExhausted,
          settled,
          timed_out: timedOut,
          polls_performed: pollsPerformed,
          max_wait_ms: maxWaitMs,
          poll_interval_ms: pollIntervalMs,
          verification_findings: findingMatches
            .filter((match) => match.found && match.finding)
            .map((match) => match.finding as VerificationFindingSummary),
          delegate_snapshots: delegateSnapshots,
          ...finalAssessment,
        };
      }
  
      await sleep(pollIntervalMs);
    }
  }
  async function resolveVerifierParentSessionId(
    rawSessionId: unknown,
    toolName: string
  ): Promise<string> {
    if (agentType === "ralphx-plan-verifier" && typeof contextId === "string" && contextId.length > 0) {
      const parentContext = await callTauriGet(`parent_session_context/${contextId}`) as {
        parent_session?: {
          id?: string;
        };
      };
      const canonicalParentId =
        typeof parentContext.parent_session?.id === "string" && parentContext.parent_session.id.length > 0
          ? parentContext.parent_session.id
          : undefined;
      if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
        const providedSessionId = rawSessionId.trim();
        if (providedSessionId === contextId && canonicalParentId) {
          return canonicalParentId;
        }
        if (canonicalParentId && providedSessionId === canonicalParentId) {
          return canonicalParentId;
        }
        return providedSessionId;
      }
      if (canonicalParentId) {
        return canonicalParentId;
      }
    }
    if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
      return rawSessionId.trim();
    }
    throw new Error(
      `${toolName} requires session_id unless it is called from an active ralphx-plan-verifier child session with a resolvable parent ideation session.`
    );
  }

  async function resolveVerificationFindingSessionId(
    rawSessionId: unknown,
    toolName: string
  ): Promise<string> {
    if (
      typeof contextId === "string" &&
      contextId.length > 0 &&
      contextType === "delegation"
    ) {
      const delegatedStatus = (await callTauriGet(
        `coordination/delegated-session/${contextId}/status`
      )) as {
        session?: {
          id?: string;
          parent_context_type?: string;
          parent_context_id?: string;
        };
      };
      const canonicalParentId =
        delegatedStatus.session?.parent_context_type === "ideation" &&
        typeof delegatedStatus.session?.parent_context_id === "string" &&
        delegatedStatus.session.parent_context_id.length > 0
          ? delegatedStatus.session.parent_context_id
          : undefined;
      if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
        const providedSessionId = rawSessionId.trim();
        if (providedSessionId === contextId && canonicalParentId) {
          return canonicalParentId;
        }
        if (canonicalParentId && providedSessionId === canonicalParentId) {
          return canonicalParentId;
        }
        return providedSessionId;
      }
      if (canonicalParentId) {
        return canonicalParentId;
      }
    }
    return resolveContextSessionId(rawSessionId, toolName);
  }
  
  function resolveContextSessionId(rawSessionId: unknown, toolName: string): string {
    if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
      return rawSessionId;
    }
    if (typeof contextId === "string" && contextId.trim().length > 0) {
      return contextId;
    }
    throw new Error(
      `${toolName} requires session_id unless the current RalphX session context is available for automatic injection.`
    );
  }

  async function assessVerificationRoundState(
    args: VerificationAssessmentArgs
  ): Promise<Record<string, unknown>> {
    const includeMessages = args.include_messages !== false;
    const messageLimit = normalizeMessageLimit(args.message_limit);
    const rescueBudgetExhausted = args.rescue_budget_exhausted === true;
    const findingMatches = await loadVerificationFindingsByCritic({
      sessionId: args.session_id,
      critics: Array.from(
        new Set(
          args.delegates
            .map((delegate) => delegate.label?.trim().toLowerCase())
            .filter((label): label is string => Boolean(label))
        )
      ),
      createdAfter: args.created_after,
    });
    const findingByCritic = new Map(
      findingMatches.map((match) => [match.critic, match] as const)
    );
    const artifactsByPrefix = Array.from(
      new Set(args.delegates.map((delegate) => delegate.artifact_prefix))
    ).map((prefix) => {
      const delegate = args.delegates.find((entry) => entry.artifact_prefix === prefix);
      const critic = delegate?.label?.trim().toLowerCase();
      const findingMatch = critic ? findingByCritic.get(critic) : undefined;
      return findingMatch?.found && findingMatch.finding
        ? {
            prefix,
            found: true,
            total_matches: findingMatch.total_matches,
            artifact: {
              id: findingMatch.finding.artifact_id,
              name: findingMatch.finding.title,
              created_at: findingMatch.finding.created_at,
            },
          }
        : {
            prefix,
            found: false,
            total_matches: 0,
          };
    });
    const delegateSnapshots = await loadVerificationDelegateSnapshots({
      delegates: args.delegates,
      includeMessages,
      messageLimit,
    });
    return {
      session_id: args.session_id,
      created_after: args.created_after ?? null,
      rescue_budget_exhausted: rescueBudgetExhausted,
      verification_findings: findingMatches
        .filter((match) => match.found && match.finding)
        .map((match) => match.finding as VerificationFindingSummary),
      ...assessVerificationRound({
        delegates: args.delegates,
        artifactsByPrefix,
        delegateSnapshots,
        rescueBudgetExhausted,
      }),
    };
  }

  async function getPlanVerificationForTool(args: {
    session_id?: string;
  }): Promise<unknown> {
    const sessionId = await resolveVerifierParentSessionId(
      args.session_id,
      "get_plan_verification"
    );
    return await callTauriGet(`ideation/sessions/${sessionId}/verification`);
  }

  async function reportVerificationRoundForTool(args: {
    session_id?: string;
    round: number;
    gaps?: unknown[];
    generation: number;
    [key: string]: unknown;
  }): Promise<unknown> {
    const { session_id: rawSessionId, ...body } = args;
    const sessionId = await resolveVerifierParentSessionId(
      rawSessionId,
      "report_verification_round"
    );
    return await callTauri(`ideation/sessions/${sessionId}/verification`, {
      ...body,
      status: "reviewing",
      in_progress: true,
    });
  }

  async function completePlanVerificationForTool(args: {
    session_id?: string;
    status: string;
    round?: number;
    gaps?: unknown[];
    convergence_reason?: string;
    generation: number;
    required_delegates?: VerificationRoundDelegateInput[];
    created_after?: string;
    rescue_budget_exhausted?: boolean;
    include_full_content?: boolean;
    include_messages?: boolean;
    message_limit?: number;
    max_wait_ms?: number;
    poll_interval_ms?: number;
  }): Promise<unknown> {
    const {
      session_id: rawSessionId,
      required_delegates,
      created_after,
      rescue_budget_exhausted = false,
      include_full_content = true,
      include_messages = true,
      message_limit = 5,
      max_wait_ms = VERIFICATION_REQUIRED_WAIT_DEFAULT_MS,
      poll_interval_ms = 750,
      ...body
    } = args;
    const sessionId = await resolveVerifierParentSessionId(
      rawSessionId,
      "complete_plan_verification"
    );
    const isVerifierTerminalNonUserUpdate =
      agentType === "ralphx-plan-verifier" &&
      body.status !== "skipped" &&
      body.convergence_reason !== "user_stopped" &&
      body.convergence_reason !== "user_skipped" &&
      body.convergence_reason !== "user_reverted";
    const hasVerifierRoundSettlementContext =
      Array.isArray(required_delegates) &&
      required_delegates.length > 0 &&
      typeof created_after === "string" &&
      created_after.length > 0;
    const isVerifierRoundTerminalUpdate =
      isVerifierTerminalNonUserUpdate && hasVerifierRoundSettlementContext;
    if (body.status === "reviewing") {
      throw new Error(
        "complete_plan_verification is terminal-only. Use verified or needs_revision here, not reviewing."
      );
    }
    if (isVerifierTerminalNonUserUpdate && !hasVerifierRoundSettlementContext) {
      return await callTauri(`ideation/sessions/${sessionId}/verification/infra-failure`, {
        generation: body.generation,
        convergence_reason: body.convergence_reason ?? "agent_error",
        round: body.round,
      });
    }

    if (Array.isArray(required_delegates) && required_delegates.length > 0) {
      return await completePlanVerificationWithSettlement({
        sessionId,
        body,
        requiredDelegates: required_delegates,
        createdAfter: created_after,
        rescueBudgetExhausted: rescue_budget_exhausted,
        includeFullContent: include_full_content,
        includeMessages: include_messages,
        messageLimit: message_limit,
        maxWaitMs: max_wait_ms,
        pollIntervalMs: poll_interval_ms,
        awaitVerificationRoundSettlement,
        callInfraFailure: async ({ generation, convergence_reason, round }) =>
          (await callTauri(`ideation/sessions/${sessionId}/verification/infra-failure`, {
            generation,
            convergence_reason: convergence_reason ?? "agent_error",
            round,
          })) as Record<string, unknown>,
        callCompletion: async (completionBody) =>
          await callTauri(`ideation/sessions/${sessionId}/verification`, completionBody),
      });
    }

    return await callTauri(`ideation/sessions/${sessionId}/verification`, {
      ...body,
      in_progress: false,
    });
  }

  async function runVerificationEnrichment(args: {
    session_id?: string;
    disabled_specialists?: string[];
    include_full_content?: boolean;
    include_messages?: boolean;
    message_limit?: number;
    max_wait_ms?: number;
    poll_interval_ms?: number;
  }): Promise<unknown> {
    const sessionId = await resolveVerifierParentSessionId(
      args.session_id,
      "run_verification_enrichment"
    );
    return await runVerificationEnrichmentPass(
      {
        loadPlanSnapshot: loadVerificationPlanSnapshot,
        startDelegate: async ({ agentName, parentSessionId, prompt, delegatedSessionId }) => {
          const launched = await startManagedVerificationDelegate({
            agentName,
            parentSessionId,
            prompt,
            delegatedSessionId,
          });
          return {
            job_id: launched.job_id,
            delegated_session_id: launched.delegated_session_id,
            agent_name: agentName,
            artifact_prefix: "",
            required: false,
          };
        },
        awaitOptionalDelegates: awaitOptionalVerificationDelegates,
        runRequiredCriticRound: runRequiredVerificationCriticRound,
      },
      {
        sessionId,
        disabledSpecialists: new Set(
          (args.disabled_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)
        ),
        includeFullContent: args.include_full_content !== false,
        includeMessages: args.include_messages !== false,
        messageLimit: normalizeMessageLimit(args.message_limit),
        maxWaitMs: normalizeMaxWaitMs(
          args.max_wait_ms,
          VERIFICATION_ENRICHMENT_WAIT_DEFAULT_MS,
          VERIFICATION_ENRICHMENT_WAIT_DEFAULT_MS
        ),
        pollIntervalMs: normalizePollIntervalMs(args.poll_interval_ms, 500),
      }
    );
  }

  async function runVerificationRound(args: {
    session_id?: string;
    round: number;
    disabled_specialists?: string[];
    include_full_content?: boolean;
    include_messages?: boolean;
    message_limit?: number;
    max_wait_ms?: number;
    optional_wait_ms?: number;
    poll_interval_ms?: number;
  }): Promise<unknown> {
    const sessionId = await resolveVerifierParentSessionId(
      args.session_id,
      "run_verification_round"
    );
    return await runVerificationRoundPass(
      {
        loadPlanSnapshot: loadVerificationPlanSnapshot,
        startDelegate: async ({ agentName, parentSessionId, prompt, delegatedSessionId }) => {
          const launched = await startManagedVerificationDelegate({
            agentName,
            parentSessionId,
            prompt,
            delegatedSessionId,
          });
          return {
            job_id: launched.job_id,
            delegated_session_id: launched.delegated_session_id,
            agent_name: agentName,
            artifact_prefix: "",
            required: false,
          };
        },
        awaitOptionalDelegates: awaitOptionalVerificationDelegates,
        runRequiredCriticRound: runRequiredVerificationCriticRound,
      },
      {
        sessionId,
        round: args.round,
        disabledSpecialists: new Set(
          (args.disabled_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)
        ),
        includeFullContent: args.include_full_content !== false,
        includeMessages: args.include_messages !== false,
        messageLimit: normalizeMessageLimit(args.message_limit),
        maxWaitMs: normalizeMaxWaitMs(args.max_wait_ms, VERIFICATION_REQUIRED_WAIT_DEFAULT_MS),
        optionalWaitMs: normalizeMaxWaitMs(
          args.optional_wait_ms,
          VERIFICATION_OPTIONAL_WAIT_DEFAULT_MS,
          VERIFICATION_OPTIONAL_WAIT_DEFAULT_MS
        ),
        pollIntervalMs: normalizePollIntervalMs(args.poll_interval_ms, 750),
      }
    );
  }

  async function runRequiredVerificationCriticRoundTool(args: {
    session_id?: string;
    round: number;
    include_full_content?: boolean;
    include_messages?: boolean;
    message_limit?: number;
    max_wait_ms?: number;
    poll_interval_ms?: number;
  }): Promise<RequiredCriticRoundResult> {
    const sessionId = await resolveVerifierParentSessionId(
      args.session_id,
      "run_required_verification_critic_round"
    );
    return await runRequiredVerificationCriticRound({
      sessionId,
      round: args.round,
      includeFullContent: args.include_full_content !== false,
      includeMessages: args.include_messages !== false,
      messageLimit: normalizeMessageLimit(args.message_limit),
      maxWaitMs: normalizeMaxWaitMs(args.max_wait_ms, VERIFICATION_REQUIRED_WAIT_DEFAULT_MS),
      pollIntervalMs: normalizePollIntervalMs(args.poll_interval_ms, 750),
    });
  }

  async function awaitVerificationRoundSettlementForTool(
    args: VerificationSettlementArgs
  ): Promise<AwaitVerificationRoundSettlementResult> {
    return await awaitVerificationRoundSettlement({
      ...args,
      session_id: await resolveVerifierParentSessionId(
        args.session_id,
        "await_verification_round_settlement"
      ),
    });
  }

  return {
    assessVerificationRoundState,
    getPlanVerificationForTool,
    reportVerificationRoundForTool,
    completePlanVerificationForTool,
    runVerificationEnrichment,
    runVerificationRound,
    runRequiredVerificationCriticRoundTool,
    awaitVerificationRoundSettlementForTool,
    selectLatestArtifactsByPrefix,
    loadVerificationFindingsByCritic,
    loadVerificationDelegateSnapshots,
    loadVerificationPlanSnapshot,
    awaitOptionalVerificationDelegates,
    startManagedVerificationDelegate,
    runRequiredVerificationCriticRound,
    awaitVerificationRoundSettlement,
    resolveVerifierParentSessionId,
    resolveVerificationFindingSessionId,
    resolveContextSessionId,
  };
}
