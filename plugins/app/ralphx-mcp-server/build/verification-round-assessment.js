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
    const terminalWithoutArtifact = jobStatus === "completed" ||
        jobStatus === "failed" ||
        jobStatus === "cancelled" ||
        runStatus === "completed" ||
        runStatus === "failed" ||
        runStatus === "cancelled";
    if (rescueBudgetExhausted) {
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
                : "Required artifact is missing and the delegate no longer appears to be actively running."),
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