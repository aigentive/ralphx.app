function mergeGapSource(current, incoming) {
    if (!current) {
        return incoming;
    }
    if (!incoming || current === incoming) {
        return current;
    }
    if (current === "both" || incoming === "both") {
        return "both";
    }
    return "both";
}
function gapDedupKey(gap) {
    return [
        gap.severity.trim().toLowerCase(),
        gap.category.trim().toLowerCase(),
        gap.description.trim().toLowerCase(),
        (gap.why_it_matters ?? "").trim().toLowerCase(),
    ].join("::");
}
export function aggregateVerificationGaps(findings) {
    const merged = new Map();
    for (const finding of findings) {
        if (!finding.usable) {
            continue;
        }
        for (const gap of finding.gaps) {
            const key = gapDedupKey(gap);
            const existing = merged.get(key);
            if (!existing) {
                merged.set(key, {
                    ...gap,
                });
                continue;
            }
            merged.set(key, {
                ...existing,
                why_it_matters: existing.why_it_matters ?? gap.why_it_matters,
                source: mergeGapSource(existing.source, gap.source),
            });
        }
    }
    const merged_gaps = Array.from(merged.values());
    const gap_counts = {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
    };
    for (const gap of merged_gaps) {
        gap_counts[gap.severity] += 1;
    }
    return {
        merged_gaps,
        gap_counts,
    };
}
function normalizeSeverity(value) {
    if (value === "critical" || value === "high" || value === "medium" || value === "low") {
        return value;
    }
    return null;
}
function normalizeSource(value) {
    if (value === "layer1" || value === "layer2" || value === "both") {
        return value;
    }
    return undefined;
}
export function parseVerificationCriticArtifact(params) {
    const base = {
        prefix: params.prefix,
        label: params.label,
        usable: false,
        artifact_id: params.artifact?.id,
        artifact_name: params.artifact?.name,
        artifact_created_at: params.artifact?.created_at,
        gaps: [],
    };
    const rawContent = params.artifact?.content;
    if (typeof rawContent !== "string" || rawContent.trim().length === 0) {
        return {
            ...base,
            parse_error: "Artifact content is missing.",
        };
    }
    let parsed;
    try {
        parsed = JSON.parse(rawContent);
    }
    catch (error) {
        return {
            ...base,
            parse_error: error instanceof Error ? error.message : "Artifact content is not valid JSON.",
        };
    }
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        return {
            ...base,
            parse_error: "Artifact JSON must be an object.",
        };
    }
    const record = parsed;
    const status = typeof record.status === "string" ? record.status : undefined;
    const critic = typeof record.critic === "string" ? record.critic : undefined;
    const round = typeof record.round === "number" ? record.round : undefined;
    const coverage = typeof record.coverage === "string" ? record.coverage : undefined;
    const summary = typeof record.summary === "string" ? record.summary : undefined;
    const rawGaps = Array.isArray(record.gaps) ? record.gaps : [];
    const gaps = [];
    for (const gap of rawGaps) {
        if (!gap || typeof gap !== "object" || Array.isArray(gap)) {
            continue;
        }
        const gapRecord = gap;
        const severity = normalizeSeverity(gapRecord.severity);
        const category = typeof gapRecord.category === "string" ? gapRecord.category.trim() : "";
        const description = typeof gapRecord.description === "string" ? gapRecord.description.trim() : "";
        if (!severity || category.length === 0 || description.length === 0) {
            continue;
        }
        const whyItMatters = typeof gapRecord.why_it_matters === "string"
            ? gapRecord.why_it_matters
            : undefined;
        gaps.push({
            severity,
            category,
            description,
            why_it_matters: whyItMatters,
            source: normalizeSource(gapRecord.source),
        });
    }
    const usable = (status === "complete" || status === "partial" || status === "error") &&
        typeof critic === "string" &&
        typeof summary === "string";
    return {
        ...base,
        usable,
        status,
        critic,
        round,
        coverage,
        summary,
        gaps,
        parse_error: usable ? undefined : "Artifact JSON is missing required verifier fields.",
    };
}
export function parseTypedVerificationFinding(params) {
    const finding = params.finding;
    const base = {
        prefix: params.label,
        label: params.label,
        usable: false,
        artifact_id: finding?.artifact_id,
        artifact_name: finding?.title,
        artifact_created_at: finding?.created_at,
        gaps: [],
    };
    if (!finding) {
        return {
            ...base,
            parse_error: "Typed verification finding is missing.",
        };
    }
    const status = finding.status === "complete" ||
        finding.status === "partial" ||
        finding.status === "error"
        ? finding.status
        : undefined;
    const gaps = [];
    for (const gap of finding.gaps ?? []) {
        const severity = normalizeSeverity(gap.severity);
        const category = typeof gap.category === "string" ? gap.category.trim() : "";
        const description = typeof gap.description === "string" ? gap.description.trim() : "";
        if (!severity || category.length === 0 || description.length === 0) {
            continue;
        }
        gaps.push({
            severity,
            category,
            description,
            why_it_matters: typeof gap.why_it_matters === "string" ? gap.why_it_matters : undefined,
            source: normalizeSource(gap.source),
        });
    }
    const usable = typeof finding.critic === "string" &&
        typeof finding.summary === "string" &&
        status !== undefined;
    return {
        ...base,
        usable,
        status,
        critic: finding.critic,
        round: finding.round,
        coverage: typeof finding.coverage === "string" ? finding.coverage : undefined,
        summary: finding.summary,
        gaps,
        parse_error: usable ? undefined : "Typed verification finding is missing required fields.",
    };
}
function isRunningLike(status) {
    return (status === "running" ||
        status === "queued" ||
        status === "likely_generating" ||
        status === "likely_waiting");
}
function classifyDelegate(delegate, artifact, snapshot, rescueBudgetExhausted) {
    const label = delegate.label || delegate.artifact_prefix.trim() || delegate.job_id;
    const required = delegate.required !== false;
    const artifactFound = artifact?.found === true;
    if (artifactFound) {
        return {
            job_id: delegate.job_id,
            label,
            artifact_prefix: delegate.artifact_prefix,
            required,
            artifact_found: true,
            assessment: "artifact_published",
            status: "completed",
            reason: "Required artifact was published for this delegate.",
        };
    }
    const jobStatus = snapshot?.status ?? "unknown";
    const runStatus = snapshot?.delegated_status?.latest_run?.status ?? null;
    const estimatedStatus = snapshot?.delegated_status?.agent_state?.estimated_status ?? null;
    const error = snapshot?.error ??
        snapshot?.delegated_status?.latest_run?.error_message ??
        null;
    if (isRunningLike(jobStatus) ||
        isRunningLike(runStatus) ||
        isRunningLike(estimatedStatus)) {
        return {
            job_id: delegate.job_id,
            label,
            artifact_prefix: delegate.artifact_prefix,
            required,
            artifact_found: false,
            assessment: "pending",
            status: jobStatus,
            reason: "Delegate still appears to be running and has not published the required artifact yet.",
        };
    }
    const terminalWithoutArtifact = jobStatus === "completed" ||
        jobStatus === "failed" ||
        jobStatus === "cancelled" ||
        runStatus === "completed" ||
        runStatus === "failed" ||
        runStatus === "cancelled";
    if (!rescueBudgetExhausted) {
        return {
            job_id: delegate.job_id,
            label,
            artifact_prefix: delegate.artifact_prefix,
            required,
            artifact_found: false,
            assessment: "pending",
            status: jobStatus,
            reason: error ??
                (terminalWithoutArtifact
                    ? "Delegate finished without publishing the required artifact yet; the bounded rescue/wait budget is still available."
                    : "Required artifact is still missing and the bounded rescue/wait budget is still available."),
        };
    }
    return {
        job_id: delegate.job_id,
        label,
        artifact_prefix: delegate.artifact_prefix,
        required,
        artifact_found: false,
        assessment: "infra_failure",
        status: jobStatus,
        reason: error ??
            (terminalWithoutArtifact
                ? "Delegate reached a terminal state without publishing the required artifact."
                : "Required artifact is still missing after the allowed wait/rescue budget."),
    };
}
export function assessVerificationRound(params) {
    const rescueBudgetExhausted = params.rescueBudgetExhausted === true;
    const artifactByPrefix = new Map(params.artifactsByPrefix.map((artifact) => [artifact.prefix, artifact]));
    const snapshotByJobId = new Map(params.delegateSnapshots.map((snapshot) => [snapshot.job_id, snapshot]));
    const delegateAssessments = params.delegates.map((delegate) => classifyDelegate(delegate, artifactByPrefix.get(delegate.artifact_prefix), snapshotByJobId.get(delegate.job_id), rescueBudgetExhausted));
    const requiredAssessments = delegateAssessments.filter((delegate) => delegate.required);
    const missingRequiredPrefixes = requiredAssessments
        .filter((delegate) => !delegate.artifact_found)
        .map((delegate) => delegate.artifact_prefix);
    const classification = missingRequiredPrefixes.length === 0
        ? "complete"
        : requiredAssessments.some((delegate) => delegate.assessment === "infra_failure")
            ? "infra_failure"
            : "pending";
    const recommendedNextAction = classification === "complete"
        ? "continue_round_analysis"
        : classification === "pending"
            ? "perform_single_rescue_or_wait"
            : "complete_verification_with_infra_failure";
    const summary = classification === "complete"
        ? "All required verification artifacts were published."
        : classification === "pending"
            ? `Required verification artifacts are still pending for: ${missingRequiredPrefixes.join(", ")}.`
            : `Required verification artifacts are missing after the allowed wait/rescue budget: ${missingRequiredPrefixes.join(", ")}.`;
    return {
        classification,
        recommended_next_action: recommendedNextAction,
        summary,
        missing_required_prefixes: missingRequiredPrefixes,
        delegate_assessments: delegateAssessments,
        artifacts_by_prefix: params.artifactsByPrefix,
    };
}
//# sourceMappingURL=verification-round-assessment.js.map