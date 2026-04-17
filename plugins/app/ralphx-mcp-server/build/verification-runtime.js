import { assessVerificationRound, } from "./verification-round-assessment.js";
import { runVerificationEnrichmentPass, runVerificationRoundPass, } from "./verification-orchestration.js";
import { completePlanVerificationWithSettlement } from "./verification-completion.js";
export function createVerificationRuntime(deps) {
    const { callTauri, callTauriGet, agentType, contextType, contextId } = deps;
    const VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS = 90 * 1000;
    const VERIFICATION_REQUIRED_WAIT_DEFAULT_MS = VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS;
    const VERIFICATION_OPTIONAL_WAIT_DEFAULT_MS = VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS;
    const VERIFICATION_ENRICHMENT_WAIT_DEFAULT_MS = VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS;
    const VERIFICATION_RESCUE_WAIT_SLICE_MS = 15 * 1000;
    const verificationRoundStateBySession = new Map();
    function normalizeMessageLimit(messageLimit) {
        return Math.min(Math.max(messageLimit ?? 5, 1), 50);
    }
    function normalizeMaxWaitMs(maxWaitMs, fallback, cap = VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS) {
        return Math.min(Math.max(maxWaitMs ?? fallback, 0), cap);
    }
    function normalizePollIntervalMs(pollIntervalMs, fallback) {
        return Math.max(pollIntervalMs ?? fallback, 100);
    }
    function isRunningLikeStatus(status) {
        return (status === "running" ||
            status === "queued" ||
            status === "likely_generating" ||
            status === "likely_waiting");
    }
    function rememberVerificationRoundState(sessionId, state) {
        verificationRoundStateBySession.set(sessionId, state);
    }
    function getVerificationRoundState(sessionId) {
        return verificationRoundStateBySession.get(sessionId);
    }
    function clearVerificationRoundState(sessionId) {
        verificationRoundStateBySession.delete(sessionId);
    }
    async function sleep(ms) {
        await new Promise((resolve) => setTimeout(resolve, ms));
    }
    function selectLatestVerificationFindingsByCritic(findings, critics, createdAfter, round) {
        const createdAfterMs = typeof createdAfter === "string" && createdAfter.length > 0
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
    async function loadVerificationFindingsByCritic(args) {
        const searchParams = new URLSearchParams();
        if (typeof args.round === "number") {
            searchParams.set("round", String(args.round));
        }
        if (typeof args.createdAfter === "string" && args.createdAfter.length > 0) {
            searchParams.set("created_after", args.createdAfter);
        }
        const query = searchParams.toString();
        const response = await callTauriGet(`team/verification-findings/${args.sessionId}${query.length > 0 ? `?${query}` : ""}`);
        return selectLatestVerificationFindingsByCritic(response.findings ?? [], args.critics, args.createdAfter, args.round);
    }
    async function loadVerificationDelegateSnapshots(args) {
        return await Promise.all(args.delegates.map(async (delegate) => {
            try {
                return await callTauri("coordination/delegate/wait", {
                    job_id: delegate.job_id,
                    include_delegated_status: true,
                    include_messages: args.includeMessages,
                    message_limit: args.messageLimit,
                });
            }
            catch (error) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                return {
                    job_id: delegate.job_id,
                    status: "failed",
                    error: errorMessage,
                };
            }
        }));
    }
    const REQUIRED_VERIFICATION_CRITICS = [
        {
            agent_name: "ralphx:ralphx-plan-critic-completeness",
            critic: "completeness",
            label: "completeness",
            initial_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nRead the current plan, stay bounded to the Affected Files plus at most one adjacent integration point per file family, then publish exactly one completeness verification finding with publish_verification_finding. Use critic='completeness'. If analysis is incomplete, publish status='partial' immediately instead of continuing to explore.`,
            rescue_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nCompleteness rescue pass. Publish the completeness verification finding now with publish_verification_finding. If analysis is partial, publish status='partial' instead of exploring further.`,
        },
        {
            agent_name: "ralphx:ralphx-plan-critic-implementation-feasibility",
            critic: "feasibility",
            label: "feasibility",
            initial_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nRead the current plan, stay bounded to the Affected Files plus at most one adjacent integration point per file family, then publish exactly one feasibility verification finding with publish_verification_finding. Use critic='feasibility'. If analysis is incomplete, publish status='partial' immediately instead of continuing to explore.`,
            rescue_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nFeasibility rescue pass. Publish the feasibility verification finding now with publish_verification_finding. If analysis is partial, publish status='partial' instead of exploring further.`,
        },
    ];
    async function loadVerificationPlanSnapshot(sessionId) {
        const planData = await callTauriGet(`get_session_plan/${sessionId}`);
        return {
            artifact_id: typeof planData.artifact_id === "string"
                ? planData.artifact_id
                : typeof planData.id === "string"
                    ? planData.id
                    : undefined,
            content: typeof planData.content === "string" ? planData.content : "",
            project_working_directory: typeof planData.project_working_directory === "string"
                ? planData.project_working_directory
                : null,
        };
    }
    async function awaitOptionalVerificationDelegates(args) {
        const deadline = Date.now() +
            Math.min(Math.max(args.maxWaitMs, 0), VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS);
        let pollsPerformed = 0;
        while (true) {
            pollsPerformed += 1;
            const findingsByCritic = await loadVerificationFindingsByCritic({
                sessionId: args.sessionId,
                critics: args.critics,
                createdAfter: args.createdAfter,
                round: undefined,
            });
            const delegateSnapshots = await loadVerificationDelegateSnapshots({
                delegates: args.delegates,
                includeMessages: args.includeMessages,
                messageLimit: args.messageLimit,
            });
            const allSettled = args.delegates.every((delegate) => {
                const findingMatch = findingsByCritic.find((entry) => entry.critic === delegate.critic);
                if (findingMatch?.found === true) {
                    return true;
                }
                const snapshot = delegateSnapshots.find((entry) => entry.job_id === delegate.job_id);
                const statuses = [
                    snapshot?.status,
                    snapshot?.delegated_status?.latest_run?.status ?? undefined,
                    snapshot?.delegated_status?.agent_state?.estimated_status ?? undefined,
                ];
                return statuses.some((status) => status === "completed" || status === "failed" || status === "cancelled");
            });
            if (allSettled || Date.now() >= deadline) {
                return {
                    created_after: args.createdAfter,
                    polls_performed: pollsPerformed,
                    timed_out: !allSettled,
                    delegates: args.delegates.map(({ job_id, critic, label, required }) => ({
                        job_id,
                        critic,
                        label,
                        required,
                    })),
                    findings_by_critic: findingsByCritic,
                    delegate_snapshots: delegateSnapshots,
                };
            }
            await sleep(args.pollIntervalMs);
        }
    }
    async function startManagedVerificationDelegate(args) {
        return await callTauri("coordination/delegate/start", {
            agent_name: args.agentName,
            parent_session_id: args.parentSessionId,
            prompt: args.prompt,
            delegated_session_id: args.delegatedSessionId,
            caller_agent_name: agentType,
            caller_context_type: contextType,
            caller_context_id: contextId,
        });
    }
    function summarizeVerificationInfraFailure(args) {
        const rescueDelegates = args.rescueDelegates ?? [];
        return {
            session_id: args.sessionId,
            round: args.round,
            created_after: args.createdAfter,
            rescue_dispatched: args.rescueDispatched === true,
            required_delegates: args.delegates.map(({ job_id, critic, label, required }) => ({
                job_id,
                critic,
                label,
                required,
            })),
            rescue_delegates: rescueDelegates.map(({ job_id, critic, label, required }) => ({
                job_id,
                critic,
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
                missing_required_critics: REQUIRED_VERIFICATION_CRITICS
                    .filter((critic) => !args.delegates.some((delegate) => delegate.critic === critic.critic))
                    .map((critic) => critic.critic),
                delegate_assessments: [],
                findings_by_critic: [],
                error: args.error ?? null,
            },
        };
    }
    async function runRequiredVerificationCriticRound(args) {
        const dispatchStartedAt = Date.now();
        const createdAfter = new Date(dispatchStartedAt - 5000).toISOString();
        const totalWaitBudgetMs = normalizeMaxWaitMs(args.maxWaitMs, VERIFICATION_REQUIRED_WAIT_DEFAULT_MS);
        const rescueWaitBudgetMs = totalWaitBudgetMs > VERIFICATION_RESCUE_WAIT_SLICE_MS
            ? VERIFICATION_RESCUE_WAIT_SLICE_MS
            : Math.max(args.pollIntervalMs, Math.floor(totalWaitBudgetMs / 2));
        const initialWaitBudgetMs = Math.max(totalWaitBudgetMs - rescueWaitBudgetMs, args.pollIntervalMs);
        const initialLaunches = await Promise.all(REQUIRED_VERIFICATION_CRITICS.map(async (critic) => {
            try {
                const launched = await startManagedVerificationDelegate({
                    agentName: critic.agent_name,
                    parentSessionId: args.sessionId,
                    prompt: critic.initial_prompt(args.sessionId, args.round),
                });
                return {
                    ok: true,
                    critic,
                    delegate: {
                        job_id: launched.job_id,
                        delegated_session_id: launched.delegated_session_id,
                        agent_name: critic.agent_name,
                        critic: critic.critic,
                        label: critic.label,
                        required: true,
                    },
                };
            }
            catch (error) {
                return {
                    ok: false,
                    critic,
                    error: error instanceof Error ? error.message : String(error),
                };
            }
        }));
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
                summary: "Required critic dispatch failed before the verification round could settle.",
                error: initialLaunchFailures
                    .map((failure) => `${failure.critic.label}: ${failure.error}`)
                    .join("; "),
            });
        }
        const initialDelegateInputs = initialDelegates.map(({ job_id, critic, label, required }) => ({
            job_id,
            critic,
            label,
            required,
        }));
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
            if (!firstSettlement.missing_required_critics.includes(delegate.critic)) {
                return false;
            }
            const snapshot = firstSettlement.delegate_snapshots.find((entry) => entry.job_id === delegate.job_id);
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
        const rescueLaunches = await Promise.all(rescueTargets.map(async (target) => {
            const critic = REQUIRED_VERIFICATION_CRITICS.find((entry) => entry.critic === target.critic);
            if (!critic) {
                return {
                    ok: false,
                    delegate: target,
                    error: `Unknown required critic ${target.critic}`,
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
                    ok: true,
                    target,
                    delegate: {
                        job_id: launched.job_id,
                        delegated_session_id: launched.delegated_session_id ?? target.delegated_session_id,
                        agent_name: critic.agent_name,
                        critic: critic.critic,
                        label: critic.label,
                        required: true,
                    },
                };
            }
            catch (error) {
                return {
                    ok: false,
                    delegate: target,
                    error: error instanceof Error ? error.message : String(error),
                };
            }
        }));
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
                summary: "A required critic rescue dispatch failed, so the verification round cannot be trusted as plan feedback.",
                error: rescueFailures
                    .map((failure) => `${failure.delegate.label ?? failure.delegate.critic}: ${failure.error}`)
                    .join("; "),
            });
        }
        const finalDelegates = initialDelegates.map((delegate) => {
            const replacement = successfulRescues.find((rescue) => rescue.critic === delegate.critic);
            return replacement ?? delegate;
        });
        const finalDelegateInputs = finalDelegates.map(({ job_id, critic, label, required }) => ({
            job_id,
            critic,
            label,
            required,
        }));
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
            rescue_delegates: successfulRescues.map(({ job_id, critic, label, required }) => ({
                job_id,
                critic,
                label,
                required,
            })),
            settlement: finalSettlement,
        };
    }
    async function awaitVerificationRoundSettlement(args) {
        const includeMessages = args.include_messages !== false;
        const messageLimit = Math.min(Math.max(args.message_limit ?? 5, 1), 50);
        const rescueBudgetExhausted = args.rescue_budget_exhausted === true;
        const maxWaitMs = Math.min(Math.max(args.max_wait_ms ?? VERIFICATION_REQUIRED_WAIT_DEFAULT_MS, 0), VERIFICATION_TOOL_WAIT_BUDGET_CAP_MS);
        const pollIntervalMs = Math.max(args.poll_interval_ms ?? 750, 100);
        const uniqueCritics = Array.from(new Set(args.delegates.map((delegate) => delegate.critic)));
        let pollsPerformed = 0;
        let timedOut = false;
        const deadline = Date.now() + maxWaitMs;
        while (true) {
            pollsPerformed += 1;
            const findingMatches = await loadVerificationFindingsByCritic({
                sessionId: args.session_id,
                critics: Array.from(new Set(args.delegates
                    .map((delegate) => delegate.critic.trim().toLowerCase())
                    .filter((critic) => Boolean(critic)))),
                createdAfter: args.created_after,
            });
            const findingByCritic = new Map(findingMatches.map((match) => [match.critic, match]));
            const findings_by_critic = uniqueCritics.map((critic) => {
                const findingMatch = findingByCritic.get(critic);
                return findingMatch?.found && findingMatch.finding
                    ? {
                        critic,
                        found: true,
                        total_matches: findingMatch.total_matches,
                        finding: findingMatch.finding,
                    }
                    : {
                        critic,
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
                findingsByCritic: findings_by_critic,
                delegateSnapshots,
                rescueBudgetExhausted,
            });
            const settled = assessment.classification !== "pending";
            if (settled || Date.now() >= deadline) {
                timedOut = !settled && assessment.classification === "pending";
                const finalAssessment = timedOut && rescueBudgetExhausted
                    ? {
                        ...assessment,
                        classification: "pending",
                        recommended_next_action: "perform_single_rescue_or_wait",
                        summary: assessment.missing_required_critics.length > 0
                            ? `Required verification delegates are still running after the current bounded wait budget: ${assessment.missing_required_critics.join(", ")}.`
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
                        .map((match) => match.finding),
                    delegate_snapshots: delegateSnapshots,
                    ...finalAssessment,
                };
            }
            await sleep(pollIntervalMs);
        }
    }
    async function resolveVerifierParentSessionId(rawSessionId, toolName) {
        if (agentType === "ralphx-plan-verifier" && typeof contextId === "string" && contextId.length > 0) {
            const parentContext = await callTauriGet(`parent_session_context/${contextId}`);
            const canonicalParentId = typeof parentContext.parent_session?.id === "string" && parentContext.parent_session.id.length > 0
                ? parentContext.parent_session.id
                : undefined;
            if (canonicalParentId) {
                return canonicalParentId;
            }
            throw new Error(`${toolName} requires an active ralphx-plan-verifier child session with a resolvable parent ideation session.`);
        }
        if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
            return rawSessionId.trim();
        }
        throw new Error(`${toolName} requires session_id unless it is called from an active ralphx-plan-verifier child session with a resolvable parent ideation session.`);
    }
    async function resolveVerificationFindingSessionId(rawSessionId, toolName) {
        if (typeof contextId === "string" &&
            contextId.length > 0 &&
            contextType === "delegation") {
            const delegatedStatus = (await callTauriGet(`coordination/delegated-session/${contextId}/status`));
            const canonicalParentId = delegatedStatus.session?.parent_context_type === "ideation" &&
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
    function resolveContextSessionId(rawSessionId, toolName) {
        if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
            return rawSessionId;
        }
        if (typeof contextId === "string" && contextId.trim().length > 0) {
            return contextId;
        }
        throw new Error(`${toolName} requires session_id unless the current RalphX session context is available for automatic injection.`);
    }
    async function getPlanVerificationForTool(args) {
        const sessionId = await resolveVerifierParentSessionId(args.session_id, "get_plan_verification");
        return await callTauriGet(`ideation/sessions/${sessionId}/verification`);
    }
    async function markVerificationRoundStartedForTool(args) {
        const verificationView = await callTauriGet(`ideation/sessions/${args.sessionId}/verification`);
        const generation = typeof verificationView.verification_generation === "number"
            ? verificationView.verification_generation
            : undefined;
        if (generation === undefined) {
            throw new Error("run_verification_round could not resolve the authoritative verification generation for round-start persistence.");
        }
        const maxRounds = typeof verificationView.max_rounds === "number"
            ? verificationView.max_rounds
            : undefined;
        await callTauri(`ideation/sessions/${args.sessionId}/verification`, {
            status: "reviewing",
            in_progress: true,
            round: args.round,
            generation,
            ...(maxRounds !== undefined ? { max_rounds: maxRounds } : {}),
        });
        return { generation, ...(maxRounds !== undefined ? { maxRounds } : {}) };
    }
    async function reportVerificationRoundForTool(args) {
        const { session_id: rawSessionId, ...body } = args;
        const sessionId = await resolveVerifierParentSessionId(rawSessionId, "report_verification_round");
        const cachedRoundState = getVerificationRoundState(sessionId);
        const gaps = cachedRoundState?.round === body.round
            ? cachedRoundState.mergedGaps
            : body.gaps;
        if (agentType === "ralphx-plan-verifier" && !Array.isArray(gaps)) {
            throw new Error("report_verification_round requires a current backend-owned run_verification_round result for the same round.");
        }
        return await callTauri(`ideation/sessions/${sessionId}/verification`, {
            ...body,
            gaps,
            status: "reviewing",
            in_progress: true,
        });
    }
    async function completePlanVerificationForTool(args) {
        const { session_id: rawSessionId, ...body } = args;
        const sessionId = await resolveVerifierParentSessionId(rawSessionId, "complete_plan_verification");
        const isVerifierTerminalNonUserUpdate = agentType === "ralphx-plan-verifier" &&
            body.status !== "skipped" &&
            body.convergence_reason !== "user_stopped" &&
            body.convergence_reason !== "user_skipped" &&
            body.convergence_reason !== "user_reverted";
        if (body.status === "reviewing") {
            throw new Error("complete_plan_verification is terminal-only. Use verified or needs_revision here, not reviewing.");
        }
        const cachedRoundState = getVerificationRoundState(sessionId);
        if (agentType === "ralphx-plan-verifier" &&
            body.status === "needs_revision" &&
            !body.convergence_reason) {
            throw new Error("complete_plan_verification cannot finalize an actionable needs_revision result without a terminal convergence_reason. Revise the plan, continue the loop, and only finish when the backend-owned verification state is truly terminal.");
        }
        if (isVerifierTerminalNonUserUpdate && !cachedRoundState) {
            return await callTauri(`ideation/sessions/${sessionId}/verification/infra-failure`, {
                generation: body.generation,
                convergence_reason: body.convergence_reason ?? "agent_error",
                round: body.round,
            });
        }
        if (cachedRoundState &&
            cachedRoundState.requiredDelegates.length > 0 &&
            isVerifierTerminalNonUserUpdate) {
            const result = await completePlanVerificationWithSettlement({
                sessionId,
                body: {
                    ...body,
                    round: body.round ?? cachedRoundState.round,
                },
                requiredDelegates: cachedRoundState.requiredDelegates,
                createdAfter: cachedRoundState.createdAfter,
                rescueBudgetExhausted: true,
                includeFullContent: true,
                includeMessages: true,
                messageLimit: 5,
                maxWaitMs: VERIFICATION_REQUIRED_WAIT_DEFAULT_MS,
                pollIntervalMs: 750,
                awaitVerificationRoundSettlement,
                callInfraFailure: async ({ generation, convergence_reason, round }) => (await callTauri(`ideation/sessions/${sessionId}/verification/infra-failure`, {
                    generation,
                    convergence_reason: convergence_reason ?? "agent_error",
                    round,
                })),
                callCompletion: async (completionBody) => await callTauri(`ideation/sessions/${sessionId}/verification`, completionBody),
            });
            clearVerificationRoundState(sessionId);
            return result;
        }
        const result = await callTauri(`ideation/sessions/${sessionId}/verification`, {
            ...body,
            in_progress: false,
        });
        clearVerificationRoundState(sessionId);
        return result;
    }
    async function runVerificationEnrichment(args) {
        const sessionId = await resolveVerifierParentSessionId(args.session_id, "run_verification_enrichment");
        return await runVerificationEnrichmentPass({
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
                    critic: "",
                    required: false,
                };
            },
            awaitOptionalDelegates: awaitOptionalVerificationDelegates,
            runRequiredCriticRound: runRequiredVerificationCriticRound,
        }, {
            sessionId,
            selectedSpecialists: new Set((args.selected_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)),
            includeFullContent: true,
            includeMessages: true,
            messageLimit: 5,
            maxWaitMs: VERIFICATION_ENRICHMENT_WAIT_DEFAULT_MS,
            pollIntervalMs: 500,
        });
    }
    async function runVerificationRound(args) {
        const sessionId = await resolveVerifierParentSessionId(args.session_id, "run_verification_round");
        const roundStart = await markVerificationRoundStartedForTool({
            sessionId,
            round: args.round,
        });
        const result = await runVerificationRoundPass({
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
                    critic: "",
                    required: false,
                };
            },
            awaitOptionalDelegates: awaitOptionalVerificationDelegates,
            runRequiredCriticRound: runRequiredVerificationCriticRound,
        }, {
            sessionId,
            round: args.round,
            selectedSpecialists: new Set((args.selected_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)),
            includeFullContent: true,
            includeMessages: true,
            messageLimit: 5,
            maxWaitMs: VERIFICATION_REQUIRED_WAIT_DEFAULT_MS,
            optionalWaitMs: VERIFICATION_OPTIONAL_WAIT_DEFAULT_MS,
            pollIntervalMs: 750,
        });
        const requiredDelegates = Array.isArray(result.required_delegates)
            ? (result.required_delegates ?? [])
            : [];
        const createdAfter = typeof result.created_after === "string"
            ? (result.created_after ?? "")
            : "";
        if (requiredDelegates.length > 0 && createdAfter.length > 0) {
            rememberVerificationRoundState(sessionId, {
                round: args.round,
                classification: result.classification ??
                    "pending",
                createdAfter,
                requiredDelegates,
                mergedGaps: Array.isArray(result.merged_gaps)
                    ? (result.merged_gaps ?? [])
                    : [],
            });
        }
        if (agentType === "ralphx-plan-verifier" &&
            (result.classification ?? "pending") === "complete") {
            const roundReport = await reportVerificationRoundForTool({
                round: args.round,
                generation: roundStart.generation,
            });
            return {
                ...result,
                round_report: roundReport,
                verification_status: typeof roundReport.status === "string" ? roundReport.status : undefined,
                verification_in_progress: typeof roundReport.in_progress === "boolean" ? roundReport.in_progress : undefined,
                verification_convergence_reason: typeof roundReport.convergence_reason === "string"
                    ? roundReport.convergence_reason
                    : undefined,
            };
        }
        return result;
    }
    return {
        getPlanVerificationForTool,
        reportVerificationRoundForTool,
        completePlanVerificationForTool,
        runVerificationEnrichment,
        runVerificationRound,
        rememberVerificationRoundState,
        loadVerificationFindingsByCritic,
        loadVerificationDelegateSnapshots,
        loadVerificationPlanSnapshot,
        awaitOptionalVerificationDelegates,
        startManagedVerificationDelegate,
        runRequiredVerificationCriticRound,
        awaitVerificationRoundSettlement,
        getVerificationRoundState,
        clearVerificationRoundState,
        resolveVerifierParentSessionId,
        resolveVerificationFindingSessionId,
        resolveContextSessionId,
    };
}
//# sourceMappingURL=verification-runtime.js.map