import { assessVerificationRound, } from "./verification-round-assessment.js";
import { runVerificationEnrichmentPass, runVerificationRoundPass, } from "./verification-orchestration.js";
export function createVerificationRuntime(deps) {
    const { callTauri, callTauriGet, agentType, contextType, contextId } = deps;
    function normalizeMessageLimit(messageLimit) {
        return Math.min(Math.max(messageLimit ?? 5, 1), 50);
    }
    function normalizeMaxWaitMs(maxWaitMs, fallback) {
        return Math.min(Math.max(maxWaitMs ?? fallback, 0), 30000);
    }
    function normalizePollIntervalMs(pollIntervalMs, fallback) {
        return Math.max(pollIntervalMs ?? fallback, 100);
    }
    function selectLatestArtifactsByPrefix(artifacts, prefixes, createdAfter) {
        const createdAfterMs = typeof createdAfter === "string" && createdAfter.length > 0
            ? Date.parse(createdAfter)
            : Number.NaN;
        const hasThreshold = Number.isFinite(createdAfterMs);
        return prefixes.map((prefix) => {
            const matches = artifacts
                .filter((artifact) => artifact.name.startsWith(prefix))
                .filter((artifact) => {
                if (!hasThreshold)
                    return true;
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
    async function sleep(ms) {
        await new Promise((resolve) => setTimeout(resolve, ms));
    }
    async function loadVerificationArtifactsByPrefix(args) {
        const teamArtifacts = await callTauriGet(`team/artifacts/${args.sessionId}`);
        const matches = selectLatestArtifactsByPrefix(teamArtifacts.artifacts ?? [], args.prefixes, args.createdAfter);
        return await Promise.all(matches.map(async (match) => {
            if (!match.artifact || !args.includeFullContent) {
                return match;
            }
            const fullArtifact = await callTauriGet(`artifact/${match.artifact.id}`);
            return {
                ...match,
                artifact: {
                    ...match.artifact,
                    content: fullArtifact.content ?? "",
                },
            };
        }));
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
            artifact_prefix: "Completeness: ",
            label: "completeness",
            initial_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nRead the current plan, stay bounded to the Affected Files plus at most one adjacent integration point per file family, then publish exactly one completeness verification finding with publish_verification_finding. Use critic='completeness'. If analysis is incomplete, publish status='partial' immediately instead of continuing to explore.`,
            rescue_prompt: (sessionId, round) => `SESSION_ID: ${sessionId}\nROUND: ${round}\nCompleteness rescue pass. Publish the completeness verification finding now with publish_verification_finding. If analysis is partial, publish status='partial' instead of exploring further.`,
        },
        {
            agent_name: "ralphx:ralphx-plan-critic-implementation-feasibility",
            critic: "feasibility",
            artifact_prefix: "Feasibility: ",
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
        const deadline = Date.now() + Math.min(Math.max(args.maxWaitMs, 0), 30000);
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
                const artifact = artifactsByPrefix.find((entry) => entry.prefix === delegate.artifact_prefix);
                if (artifact?.found === true) {
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
                    delegates: args.delegates.map(({ job_id, artifact_prefix, label, required }) => ({
                        job_id,
                        artifact_prefix,
                        label,
                        required,
                    })),
                    artifacts_by_prefix: artifactsByPrefix,
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
                    .filter((critic) => !args.delegates.some((delegate) => delegate.artifact_prefix === critic.artifact_prefix))
                    .map((critic) => critic.artifact_prefix),
                delegate_assessments: [],
                artifacts_by_prefix: [],
                error: args.error ?? null,
            },
        };
    }
    async function runRequiredVerificationCriticRound(args) {
        const dispatchStartedAt = Date.now();
        const createdAfter = new Date(dispatchStartedAt - 5000).toISOString();
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
                        artifact_prefix: critic.artifact_prefix,
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
        const initialDelegateInputs = initialDelegates.map(({ job_id, artifact_prefix, label, required }) => ({
            job_id,
            artifact_prefix,
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
            max_wait_ms: args.maxWaitMs,
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
        const missingPrefixes = new Set(firstSettlement.missing_required_prefixes);
        const rescueTargets = initialDelegates.filter((delegate) => missingPrefixes.has(delegate.artifact_prefix));
        const rescueLaunches = await Promise.all(rescueTargets.map(async (target) => {
            const critic = REQUIRED_VERIFICATION_CRITICS.find((entry) => entry.artifact_prefix === target.artifact_prefix);
            if (!critic) {
                return {
                    ok: false,
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
                    ok: true,
                    target,
                    delegate: {
                        job_id: launched.job_id,
                        delegated_session_id: launched.delegated_session_id ?? target.delegated_session_id,
                        agent_name: critic.agent_name,
                        artifact_prefix: critic.artifact_prefix,
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
                    .map((failure) => `${failure.delegate.label ?? failure.delegate.artifact_prefix}: ${failure.error}`)
                    .join("; "),
            });
        }
        const finalDelegates = initialDelegates.map((delegate) => {
            const replacement = successfulRescues.find((rescue) => rescue.artifact_prefix === delegate.artifact_prefix);
            return replacement ?? delegate;
        });
        const finalDelegateInputs = finalDelegates.map(({ job_id, artifact_prefix, label, required }) => ({
            job_id,
            artifact_prefix,
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
            max_wait_ms: args.maxWaitMs,
            poll_interval_ms: args.pollIntervalMs,
        });
        return {
            session_id: args.sessionId,
            round: args.round,
            created_after: createdAfter,
            rescue_dispatched: true,
            required_delegates: finalDelegateInputs,
            rescue_delegates: successfulRescues.map(({ job_id, artifact_prefix, label, required }) => ({
                job_id,
                artifact_prefix,
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
        const maxWaitMs = Math.min(Math.max(args.max_wait_ms ?? 8000, 0), 30000);
        const pollIntervalMs = Math.max(args.poll_interval_ms ?? 750, 100);
        const uniquePrefixes = Array.from(new Set(args.delegates.map((delegate) => delegate.artifact_prefix)));
        let pollsPerformed = 0;
        let timedOut = false;
        const deadline = Date.now() + maxWaitMs;
        while (true) {
            pollsPerformed += 1;
            const findingMatches = await loadVerificationFindingsByCritic({
                sessionId: args.session_id,
                critics: Array.from(new Set(args.delegates
                    .map((delegate) => delegate.label?.trim().toLowerCase())
                    .filter((label) => Boolean(label)))),
                createdAfter: args.created_after,
            });
            const findingByCritic = new Map(findingMatches.map((match) => [match.critic, match]));
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
                const finalAssessment = timedOut && rescueBudgetExhausted
                    ? {
                        ...assessment,
                        classification: "infra_failure",
                        recommended_next_action: "complete_verification_with_infra_failure",
                        summary: assessment.missing_required_prefixes.length > 0
                            ? `Required verification artifacts are still missing after waiting for delegates to either publish artifacts or reach terminal state: ${assessment.missing_required_prefixes.join(", ")}.`
                            : "Required verification delegates did not settle before the terminal wait budget expired.",
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
                    ...finalAssessment,
                };
            }
            await sleep(pollIntervalMs);
        }
    }
    async function resolveVerifierParentSessionId(rawSessionId, toolName) {
        if (typeof rawSessionId === "string" && rawSessionId.trim().length > 0) {
            return rawSessionId;
        }
        if (agentType === "ralphx-plan-verifier" && typeof contextId === "string" && contextId.length > 0) {
            const parentContext = await callTauriGet(`parent_session_context/${contextId}`);
            if (typeof parentContext.parent_session?.id === "string" && parentContext.parent_session.id.length > 0) {
                return parentContext.parent_session.id;
            }
        }
        throw new Error(`${toolName} requires session_id unless it is called from an active ralphx-plan-verifier child session with a resolvable parent ideation session.`);
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
    async function assessVerificationRoundState(args) {
        const includeMessages = args.include_messages !== false;
        const messageLimit = normalizeMessageLimit(args.message_limit);
        const rescueBudgetExhausted = args.rescue_budget_exhausted === true;
        const findingMatches = await loadVerificationFindingsByCritic({
            sessionId: args.session_id,
            critics: Array.from(new Set(args.delegates
                .map((delegate) => delegate.label?.trim().toLowerCase())
                .filter((label) => Boolean(label)))),
            createdAfter: args.created_after,
        });
        const findingByCritic = new Map(findingMatches.map((match) => [match.critic, match]));
        const artifactsByPrefix = Array.from(new Set(args.delegates.map((delegate) => delegate.artifact_prefix))).map((prefix) => {
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
                .map((match) => match.finding),
            ...assessVerificationRound({
                delegates: args.delegates,
                artifactsByPrefix,
                delegateSnapshots,
                rescueBudgetExhausted,
            }),
        };
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
                    artifact_prefix: "",
                    required: false,
                };
            },
            awaitOptionalDelegates: awaitOptionalVerificationDelegates,
            runRequiredCriticRound: runRequiredVerificationCriticRound,
        }, {
            sessionId,
            disabledSpecialists: new Set((args.disabled_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)),
            includeFullContent: args.include_full_content !== false,
            includeMessages: args.include_messages !== false,
            messageLimit: normalizeMessageLimit(args.message_limit),
            maxWaitMs: normalizeMaxWaitMs(args.max_wait_ms, 4000),
            pollIntervalMs: normalizePollIntervalMs(args.poll_interval_ms, 500),
        });
    }
    async function runVerificationRound(args) {
        const sessionId = await resolveVerifierParentSessionId(args.session_id, "run_verification_round");
        return await runVerificationRoundPass({
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
        }, {
            sessionId,
            round: args.round,
            disabledSpecialists: new Set((args.disabled_specialists ?? []).map((entry) => entry.trim()).filter(Boolean)),
            includeFullContent: args.include_full_content !== false,
            includeMessages: args.include_messages !== false,
            messageLimit: normalizeMessageLimit(args.message_limit),
            maxWaitMs: normalizeMaxWaitMs(args.max_wait_ms, 8000),
            optionalWaitMs: normalizeMaxWaitMs(args.optional_wait_ms, 4000),
            pollIntervalMs: normalizePollIntervalMs(args.poll_interval_ms, 750),
        });
    }
    async function runRequiredVerificationCriticRoundTool(args) {
        const sessionId = await resolveVerifierParentSessionId(args.session_id, "run_required_verification_critic_round");
        return await runRequiredVerificationCriticRound({
            sessionId,
            round: args.round,
            includeFullContent: args.include_full_content !== false,
            includeMessages: args.include_messages !== false,
            messageLimit: normalizeMessageLimit(args.message_limit),
            maxWaitMs: normalizeMaxWaitMs(args.max_wait_ms, 8000),
            pollIntervalMs: normalizePollIntervalMs(args.poll_interval_ms, 750),
        });
    }
    async function awaitVerificationRoundSettlementForTool(args) {
        return await awaitVerificationRoundSettlement({
            ...args,
            session_id: await resolveVerifierParentSessionId(args.session_id, "await_verification_round_settlement"),
        });
    }
    return {
        assessVerificationRoundState,
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
        resolveContextSessionId,
    };
}
//# sourceMappingURL=verification-runtime.js.map