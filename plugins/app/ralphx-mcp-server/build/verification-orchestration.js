import { aggregateVerificationGaps, parseTypedVerificationFinding, } from "./verification-round-assessment.js";
function extractAffectedFilesSection(planContent) {
    const match = planContent.match(/^## Affected Files[\s\S]*?(?=^##\s|\Z)/m);
    return match?.[0] ?? "";
}
function hasExistingFileMutations(planContent) {
    const affectedFiles = extractAffectedFilesSection(planContent);
    return affectedFiles.length > 0 && /(modify|update|change|edit)\b/i.test(affectedFiles);
}
function planMatchesAny(planContent, patterns) {
    return patterns.some((pattern) => pattern.test(planContent));
}
const OPTIONAL_VERIFICATION_SPECIALISTS = [
    {
        name: "ux",
        agent_name: "ralphx:ralphx-ideation-specialist-ux",
        critic: "ux",
        label: "ux",
        applies: (plan) => planMatchesAny(plan.content, [
            /frontend\//i,
            /\.tsx?\b/i,
            /\.css\b/i,
            /\bcomponent\b/i,
            /\bui\b/i,
            /\buser flow\b/i,
        ]),
        prompt: (sessionId) => `SESSION_ID: ${sessionId}\nAnalyze the current implementation plan for UX and flow risks. Read the plan via get_session_plan(session_id: ${sessionId}). Publish exactly one verification finding with publish_verification_finding using critic='ux'. Omit session_id. If no material UX gaps exist, publish a complete finding with gaps=[].`,
    },
    {
        name: "prompt-quality",
        agent_name: "ralphx:ralphx-ideation-specialist-prompt-quality",
        critic: "prompt-quality",
        label: "prompt-quality",
        applies: (plan) => planMatchesAny(plan.content, [
            /agents\//i,
            /prompt\.md/i,
            /agent\.yaml/i,
            /\bprompt\b/i,
            /\bharness\b/i,
        ]),
        prompt: (sessionId) => `SESSION_ID: ${sessionId}\nAnalyze the current implementation plan for prompt and agent-contract risks. Read the plan via get_session_plan(session_id: ${sessionId}). Publish exactly one verification finding with publish_verification_finding using critic='prompt-quality'. Omit session_id. If no material prompt-quality gaps exist, publish a complete finding with gaps=[].`,
    },
    {
        name: "pipeline-safety",
        agent_name: "ralphx:ralphx-ideation-specialist-pipeline-safety",
        critic: "pipeline-safety",
        label: "pipeline-safety",
        applies: (plan) => planMatchesAny(plan.content, [
            /\bverification\b/i,
            /\bpipeline\b/i,
            /\borchestration\b/i,
            /\bstream/i,
            /\bmerge\b/i,
            /\bcoordination\b/i,
            /chat_service/i,
            /scheduler/i,
        ]),
        prompt: (sessionId) => `SESSION_ID: ${sessionId}\nAnalyze the current implementation plan for orchestration, streaming, merge, and side-effect safety risks. Read the plan via get_session_plan(session_id: ${sessionId}). Publish exactly one verification finding with publish_verification_finding using critic='pipeline-safety'. Omit session_id. If no material pipeline-safety gaps exist, publish a complete finding with gaps=[].`,
    },
    {
        name: "state-machine",
        agent_name: "ralphx:ralphx-ideation-specialist-state-machine",
        critic: "state-machine",
        label: "state-machine",
        applies: (plan) => planMatchesAny(plan.content, [
            /\bstate machine\b/i,
            /\btransition\b/i,
            /state_machine/i,
            /task_transition_service/i,
            /\bon_enter\b/i,
        ]),
        prompt: (sessionId) => `SESSION_ID: ${sessionId}\nAnalyze the current implementation plan for state-machine and transition-safety risks. Read the plan via get_session_plan(session_id: ${sessionId}). Publish exactly one verification finding with publish_verification_finding using critic='state-machine'. Omit session_id. If no material state-machine gaps exist, publish a complete finding with gaps=[].`,
    },
];
const VERIFICATION_ENRICHMENT_SPECIALISTS = [
    {
        name: "intent",
        agent_name: "ralphx:ralphx-ideation-specialist-intent",
        critic: "intent",
        label: "intent",
        applies: () => true,
        prompt: (sessionId) => `SESSION_ID: ${sessionId}\nAnalyze intent alignment using the plan's ## Goal section as the source of truth. Read the plan via get_session_plan(session_id: ${sessionId}). Compare the rest of the plan against that goal. Do not treat later parent chat messages like "please run verify" as a replacement product request. If misalignment exists, publish one verification finding with publish_verification_finding using critic='intent'. Omit session_id. If intent is aligned, return exactly: Intent aligned — no artifact created`,
    },
    {
        name: "code-quality",
        agent_name: "ralphx:ralphx-ideation-specialist-code-quality",
        critic: "code-quality",
        label: "code-quality",
        applies: (plan) => hasExistingFileMutations(plan.content),
        prompt: (sessionId) => `SESSION_ID: ${sessionId}\nAnalyze the code paths referenced in the plan's Affected Files section. Read the plan via get_session_plan(session_id: ${sessionId}). For each existing file being modified, identify quality improvement opportunities and publish exactly one verification finding with publish_verification_finding using critic='code-quality'. Omit session_id. If no material code-quality gaps exist, publish a complete finding with gaps=[].`,
    },
];
export async function runVerificationEnrichmentPass(deps, args) {
    const plan = await deps.loadPlanSnapshot(args.sessionId);
    if (plan.content.trim().length === 0) {
        throw new Error("Verification enrichment requires an existing plan.");
    }
    const selected = VERIFICATION_ENRICHMENT_SPECIALISTS.filter((specialist) => args.selectedSpecialists.has(specialist.name));
    const createdAfter = new Date(Date.now() - 5000).toISOString();
    const launches = await Promise.all(selected.map(async (specialist) => ({
        ...(await deps.startDelegate({
            agentName: specialist.agent_name,
            parentSessionId: args.sessionId,
            prompt: specialist.prompt(args.sessionId),
        })),
        critic: specialist.critic,
        label: specialist.label,
        required: false,
    })));
    const settled = await deps.awaitOptionalDelegates({
        delegates: launches,
        sessionId: args.sessionId,
        createdAfter,
        critics: selected.map((specialist) => specialist.critic),
        includeFullContent: args.includeFullContent,
        includeMessages: args.includeMessages,
        messageLimit: args.messageLimit,
        maxWaitMs: args.maxWaitMs,
        pollIntervalMs: args.pollIntervalMs,
    });
    return {
        session_id: args.sessionId,
        requested_specialists: Array.from(args.selectedSpecialists.values()),
        selected_specialists: selected.map((specialist) => ({
            name: specialist.name,
            label: specialist.label,
            critic: specialist.critic,
            agent_name: specialist.agent_name,
        })),
        ...settled,
    };
}
export async function runVerificationRoundPass(deps, args) {
    const plan = await deps.loadPlanSnapshot(args.sessionId);
    if (plan.content.trim().length === 0) {
        throw new Error("Verification round requires an existing plan.");
    }
    const optionalSpecialists = OPTIONAL_VERIFICATION_SPECIALISTS.filter((specialist) => args.selectedSpecialists.has(specialist.name));
    const optionalLaunches = await Promise.all(optionalSpecialists.map(async (specialist) => ({
        ...(await deps.startDelegate({
            agentName: specialist.agent_name,
            parentSessionId: args.sessionId,
            prompt: specialist.prompt(args.sessionId),
        })),
        critic: specialist.critic,
        label: specialist.label,
        required: false,
    })));
    const requiredRound = await deps.runRequiredCriticRound({
        sessionId: args.sessionId,
        round: args.round,
        includeFullContent: args.includeFullContent,
        includeMessages: args.includeMessages,
        messageLimit: args.messageLimit,
        maxWaitMs: args.maxWaitMs,
        pollIntervalMs: args.pollIntervalMs,
    });
    const optionalSpecialistPayload = optionalSpecialists.map((specialist) => ({
        name: specialist.name,
        label: specialist.label,
        critic: specialist.critic,
        agent_name: specialist.agent_name,
    }));
    if (requiredRound.settlement.classification !== "complete") {
        return {
            session_id: args.sessionId,
            round: args.round,
            created_after: requiredRound.created_after,
            classification: requiredRound.settlement.classification,
            required_delegates: requiredRound.required_delegates,
            rescue_delegates: requiredRound.rescue_delegates ?? [],
            required_critic_settlement: requiredRound.settlement,
            required_findings: [],
            merged_gaps: [],
            gap_counts: {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
            optional_specialists: optionalSpecialistPayload,
            optional_delegates: optionalLaunches.map(({ job_id, critic, label, required }) => ({
                job_id,
                critic,
                label,
                required,
            })),
            optional_findings_by_critic: [],
            optional_delegate_snapshots: [],
        };
    }
    const requiredFindings = [
        parseTypedVerificationFinding({
            label: "completeness",
            finding: requiredRound.settlement.verification_findings?.find((entry) => entry.critic.trim().toLowerCase() === "completeness"),
        }),
        parseTypedVerificationFinding({
            label: "feasibility",
            finding: requiredRound.settlement.verification_findings?.find((entry) => entry.critic.trim().toLowerCase() === "feasibility"),
        }),
    ];
    const unusableRequired = requiredFindings.filter((finding) => !finding.usable);
    if (unusableRequired.length > 0) {
        return {
            session_id: args.sessionId,
            round: args.round,
            created_after: requiredRound.created_after,
            classification: "infra_failure",
            required_delegates: requiredRound.required_delegates,
            rescue_delegates: requiredRound.rescue_delegates ?? [],
            required_critic_settlement: {
                ...requiredRound.settlement,
                classification: "infra_failure",
                recommended_next_action: "complete_verification_with_infra_failure",
                summary: `Required critic findings were published but unusable: ${unusableRequired
                    .map((finding) => finding.label)
                    .join(", ")}.`,
            },
            required_findings: requiredFindings,
            merged_gaps: [],
            gap_counts: {
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
            },
            optional_specialists: optionalSpecialistPayload,
            optional_delegates: optionalLaunches.map(({ job_id, critic, label, required }) => ({
                job_id,
                critic,
                label,
                required,
            })),
            optional_findings_by_critic: [],
            optional_delegate_snapshots: [],
        };
    }
    const { merged_gaps, gap_counts } = aggregateVerificationGaps(requiredFindings);
    const optionalSettled = await deps.awaitOptionalDelegates({
        delegates: optionalLaunches,
        sessionId: args.sessionId,
        createdAfter: requiredRound.created_after,
        critics: optionalSpecialists.map((specialist) => specialist.critic),
        includeFullContent: args.includeFullContent,
        includeMessages: args.includeMessages,
        messageLimit: args.messageLimit,
        maxWaitMs: args.optionalWaitMs,
        pollIntervalMs: args.pollIntervalMs,
    });
    return {
        session_id: args.sessionId,
        round: args.round,
        created_after: requiredRound.created_after,
        classification: "complete",
        required_delegates: requiredRound.required_delegates,
        rescue_delegates: requiredRound.rescue_delegates ?? [],
        required_critic_settlement: requiredRound.settlement,
        required_findings: requiredFindings,
        merged_gaps,
        gap_counts,
        optional_specialists: optionalSpecialistPayload,
        optional_delegates: optionalSettled.delegates,
        optional_findings_by_critic: optionalSettled.findings_by_critic,
        optional_delegate_snapshots: optionalSettled.delegate_snapshots,
    };
}
//# sourceMappingURL=verification-orchestration.js.map