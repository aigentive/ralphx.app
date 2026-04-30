import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, SearchCheck } from "lucide-react";
import {
  solutionCriticApi,
  solutionCriticQueryKeys,
  type CompiledContextReadResponse,
  type ProjectedCritiqueGap,
  type ProjectedCritiqueGapActionResponse,
  type SolutionCritiqueReadResponse,
  type SolutionCritiqueTargetInput,
} from "@/api/solution-critic";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { verificationStatusKey } from "@/hooks/useVerificationStatus";
import { buildCritiqueDigest, formatCritiqueEnum } from "./critiqueDigest";
import { SolutionCritiqueDetails } from "./SolutionCritiqueDetails";

interface SolutionCritiqueActionProps {
  sessionId: string | null | undefined;
  target: SolutionCritiqueTargetInput;
  label?: string;
  className?: string;
  size?: "sm" | "xs";
  align?: "start" | "center" | "end";
  onSendToChat?: ((message: string) => void | Promise<void>) | undefined;
}

function humanize(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

type GapActionKind = "promoted" | "deferred" | "covered" | "reopened";

function applyGapActionResult(
  result: SolutionCritiqueReadResponse,
  actionResult: ProjectedCritiqueGapActionResponse,
): SolutionCritiqueReadResponse {
  const gap = actionResult.gap;
  const existingItems = result.projectedGapItems.length > 0
    ? result.projectedGapItems
    : [];
  const projectedGapItems = existingItems.some((item) => item.id === gap.id)
    ? existingItems.map((item) => (item.id === gap.id ? gap : item))
    : [...existingItems, gap];

  return {
    ...result,
    projectedGapItems,
    projectedGaps: projectedGapItems.length > 0
      ? projectedGapItems.map((item) => item.verificationGap)
      : result.projectedGaps,
  };
}

function timestamp(value: string | null | undefined): number {
  if (!value) return 0;
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? 0 : parsed;
}

function messageFromUnknownError(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message.trim()) return err.message;
  if (typeof err === "string" && err.trim()) return err;
  if (err && typeof err === "object" && "message" in err) {
    const message = (err as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
}

function newestCritiqueResult(
  local: SolutionCritiqueReadResponse | null | undefined,
  remote: SolutionCritiqueReadResponse | null | undefined,
): SolutionCritiqueReadResponse | null {
  if (!local) return remote ?? null;
  if (!remote) return local;
  return timestamp(remote.solutionCritique.generatedAt) > timestamp(local.solutionCritique.generatedAt)
    ? remote
    : local;
}

function newestCompiledContext(
  local: CompiledContextReadResponse | null | undefined,
  remote: CompiledContextReadResponse | null | undefined,
): CompiledContextReadResponse | null {
  if (!local) return remote ?? null;
  if (!remote) return local;
  return timestamp(remote.compiledContext.generatedAt) > timestamp(local.compiledContext.generatedAt)
    ? remote
    : local;
}

function truncateForChat(value: string | null | undefined, maxLength = 260): string | null {
  if (!value) return null;
  const trimmed = value.trim();
  if (!trimmed) return null;
  if (trimmed.length <= maxLength) return trimmed;
  return `${trimmed.slice(0, maxLength - 3).trimEnd()}...`;
}

function critiqueLines(
  title: string,
  items: string[],
  maxItems = 3,
): string[] {
  if (items.length === 0) return [];
  return [
    `${title}:`,
    ...items.slice(0, maxItems).map((item) => `- ${item}`),
    ...(items.length > maxItems ? [`- ${items.length - maxItems} more not included here.`] : []),
  ];
}

function buildCritiqueChatMessage(
  result: SolutionCritiqueReadResponse,
  targetLabel: string,
): string {
  const critique = result.solutionCritique;
  const safeNextAction = truncateForChat(critique.safeNextAction, 360);
  const flaggedClaims = critique.claims
    .filter((claim) => claim.status !== "supported")
    .map((claim) => {
      const claimText = truncateForChat(claim.claim);
      const noteText = truncateForChat(claim.notes, 180);
      return [
        `[${formatCritiqueEnum(claim.status)}] ${claimText ?? "Claim needs review."}`,
        noteText ? `Why: ${noteText}` : null,
      ].filter(Boolean).join(" ");
    });
  const risks = critique.risks.map((risk) => {
    const riskText = truncateForChat(risk.risk, 180);
    const mitigation = truncateForChat(risk.mitigation, 140);
    return [
      `[${formatCritiqueEnum(risk.severity)}] ${riskText ?? "Risk needs review."}`,
      mitigation ? `Mitigation: ${mitigation}` : null,
    ].filter(Boolean).join(" ");
  });
  const recommendations = critique.recommendations.map((recommendation) => {
    const recommendationText = truncateForChat(recommendation.recommendation, 180);
    const rationale = truncateForChat(recommendation.rationale, 140);
    return [
      `[${formatCritiqueEnum(recommendation.status)}] ${recommendationText ?? "Recommendation needs review."}`,
      rationale ? `Rationale: ${rationale}` : null,
    ].filter(Boolean).join(" ");
  });
  const projectedGapItems = result.projectedGapItems.length > 0
    ? result.projectedGapItems.map((gap) => gap.verificationGap)
    : result.projectedGaps;
  const projectedGaps = projectedGapItems.map((gap) => {
    const description = truncateForChat(gap.description, 180);
    const whyItMatters = truncateForChat(gap.whyItMatters, 140);
    return [
      `[${formatCritiqueEnum(gap.severity)} ${formatCritiqueEnum(gap.category)}] ${description ?? "Gap needs review."}`,
      whyItMatters ? `Why: ${whyItMatters}` : null,
    ].filter(Boolean).join(" ");
  });

  return [
    `Act on the latest solution critique for ${targetLabel}.`,
    "",
    `Verdict: ${formatCritiqueEnum(critique.verdict)}`,
    `Confidence: ${formatCritiqueEnum(critique.confidence)}`,
    ...(safeNextAction ? [`Safe next action: ${safeNextAction}`] : []),
    "",
    ...critiqueLines("Flagged claims", flaggedClaims),
    ...(flaggedClaims.length > 0 ? [""] : []),
    ...critiqueLines("Risks", risks),
    ...(risks.length > 0 ? [""] : []),
    ...critiqueLines("Recommendations", recommendations),
    ...(recommendations.length > 0 ? [""] : []),
    ...critiqueLines("Projected verification gaps", projectedGaps),
    ...(projectedGaps.length > 0 ? [""] : []),
    "Update the plan, proposal, or implementation as needed. If no change is needed, explain why.",
  ].filter((line) => line !== null).join("\n").trim();
}

export function SolutionCritiqueAction({
  sessionId,
  target,
  label = "Critique this",
  className,
  size = "sm",
  align = "end",
  onSendToChat,
}: SolutionCritiqueActionProps) {
  const [open, setOpen] = useState(false);
  const [result, setResult] = useState<SolutionCritiqueReadResponse | null>(null);
  const [context, setContext] = useState<CompiledContextReadResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const targetQueryKey = solutionCriticQueryKeys.targetCritique(sessionId, target);
  const targetContextQueryKey = solutionCriticQueryKeys.targetContext(sessionId, target);

  const latestContextQuery = useQuery({
    queryKey: targetContextQueryKey,
    queryFn: () => solutionCriticApi.getLatestTargetCompiledContext(sessionId!, target),
    enabled: Boolean(sessionId),
    staleTime: 30_000,
    retry: false,
  });
  const latestCritiqueQuery = useQuery({
    queryKey: targetQueryKey,
    queryFn: () => solutionCriticApi.getLatestTargetSolutionCritique(sessionId!, target),
    enabled: Boolean(sessionId),
    staleTime: 30_000,
    retry: false,
  });
  const activeContext = newestCompiledContext(context, latestContextQuery.data);
  const activeResult = newestCritiqueResult(result, latestCritiqueQuery.data);
  const targetLabel = target.label ?? humanize(target.targetType);

  const mutation = useMutation({
    mutationFn: async () => {
      if (!sessionId) {
        throw new Error("No ideation session is available for this critique target.");
      }
      const compiledContext = await solutionCriticApi.compileTargetContext(sessionId, target);
      const critique = await solutionCriticApi.critiqueTarget(sessionId, target, compiledContext.artifactId);
      return { compiledContext, critique };
    },
    onMutate: () => {
      setError(null);
      setOpen(true);
    },
    onSuccess: (response) => {
      setContext(response.compiledContext);
      setResult(response.critique);
      queryClient.setQueryData(targetContextQueryKey, response.compiledContext);
      queryClient.setQueryData(targetQueryKey, response.critique);
      void queryClient.invalidateQueries({ queryKey: solutionCriticQueryKeys.session(sessionId) });
    },
    onError: (err) => {
      setError(messageFromUnknownError(err, "Failed to run solution critique"));
    },
  });

  const gapActionMutation = useMutation({
    mutationFn: async ({
      critiqueArtifactId,
      gapId,
      action,
    }: {
      critiqueArtifactId: string;
      gapId: string;
      action: GapActionKind;
    }) => {
      if (!sessionId) {
        throw new Error("No ideation session is available for this critique target.");
      }
      return solutionCriticApi.applyProjectedGapAction(
        sessionId,
        critiqueArtifactId,
        gapId,
        action,
      );
    },
    onMutate: () => {
      setError(null);
    },
    onSuccess: (response) => {
      const cached = queryClient.getQueryData<SolutionCritiqueReadResponse>(targetQueryKey) ?? null;
      const base = result ?? latestCritiqueQuery.data ?? cached;
      if (base) {
        const updated = applyGapActionResult(base, response);
        setResult(updated);
        queryClient.setQueryData(targetQueryKey, updated);
      }
      void queryClient.invalidateQueries({ queryKey: targetQueryKey });
      void queryClient.invalidateQueries({ queryKey: solutionCriticQueryKeys.session(sessionId) });
      void queryClient.invalidateQueries({ queryKey: solutionCriticQueryKeys.rollup(sessionId) });
      if (response.verificationUpdated && sessionId) {
        void queryClient.invalidateQueries({ queryKey: verificationStatusKey(sessionId) });
      }
    },
    onError: (err) => {
      setError(messageFromUnknownError(err, "Failed to update critique gap"));
    },
  });

  const sendToChatMutation = useMutation({
    mutationFn: async () => {
      if (!activeResult) {
        throw new Error("No critique is available to send to chat.");
      }
      if (!onSendToChat) {
        throw new Error("No chat input is available for this critique.");
      }
      await onSendToChat(buildCritiqueChatMessage(activeResult, targetLabel));
    },
    onMutate: () => {
      setError(null);
    },
    onError: (err) => {
      setError(messageFromUnknownError(err, "Failed to send critique to chat"));
    },
  });

  if (!sessionId) return null;

  const isCompact = size === "xs";
  const latestError =
    latestContextQuery.error instanceof Error
      ? latestContextQuery.error.message
      : latestCritiqueQuery.error instanceof Error
        ? latestCritiqueQuery.error.message
        : null;
  const activeError = error ?? latestError;
  const isLoading = mutation.isPending;
  const digest = buildCritiqueDigest({
    context: activeContext,
    result: activeResult,
    isLoading,
    error: activeError,
  });
  const buttonLabel = isLoading ? "Critiquing" : activeResult ? digest.pillLabel : label;
  const handleProjectedGapAction = (gap: ProjectedCritiqueGap, action: GapActionKind) => {
    const critiqueArtifactId = activeResult?.artifactId;
    if (!critiqueArtifactId) return;
    gapActionMutation.mutate({
      critiqueArtifactId,
      gapId: gap.id,
      action,
    });
  };

  return (
    <>
      <Button
        type="button"
        variant="ghost"
        size="sm"
        className={cn(
          "gap-1.5 rounded-md text-text-primary/55 hover:bg-[var(--overlay-moderate)] hover:text-text-primary/85",
          activeResult && "border border-[var(--overlay-weak)] bg-[var(--overlay-faint)]",
          isCompact && "h-6 px-1.5 py-0 text-[10px]",
          className,
        )}
        onClick={(event) => {
          event.stopPropagation();
          const cachedResult =
            activeResult ??
            queryClient.getQueryData<SolutionCritiqueReadResponse>(targetQueryKey) ??
            null;
          const cachedContext =
            activeContext ??
            queryClient.getQueryData<CompiledContextReadResponse>(targetContextQueryKey) ??
            null;
          setError(null);
          if (cachedResult) {
            setResult(cachedResult);
            setContext(cachedContext);
            setOpen(true);
            return;
          }
          setOpen(true);
          mutation.mutate();
        }}
        disabled={mutation.isPending}
        data-testid="solution-critique-action"
        aria-label={
          activeResult
            ? `Open critique: ${formatCritiqueEnum(activeResult.solutionCritique.verdict)}`
            : label
        }
      >
        {mutation.isPending ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
        ) : (
          <SearchCheck className="h-3.5 w-3.5" />
        )}
        <span>{buttonLabel}</span>
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent
          hideCloseButton={false}
          className={cn(
            "left-auto right-4 top-4 h-[calc(100vh-2rem)] max-h-[calc(100vh-2rem)] w-[min(560px,calc(100vw-2rem))] max-w-none translate-x-0 translate-y-0 overflow-hidden p-0",
            align === "start" && "right-auto left-4",
          )}
          onClick={(event) => event.stopPropagation()}
        >
          <DialogTitle className="sr-only">Solution critique</DialogTitle>
          <DialogDescription className="sr-only">
            Evidence, risks, verification plan, and safe next action for the selected target.
          </DialogDescription>
          <SolutionCritiqueDetails
            targetLabel={targetLabel}
            context={activeContext}
            result={activeResult}
            digest={digest}
            isLoading={isLoading}
            error={activeError}
            onRefresh={() => mutation.mutate()}
            onGapAction={handleProjectedGapAction}
            pendingGapActionId={gapActionMutation.variables?.gapId ?? null}
            onSendToChat={
              onSendToChat && activeResult
                ? () => sendToChatMutation.mutate()
                : undefined
            }
            isSendingToChat={sendToChatMutation.isPending}
          />
        </DialogContent>
      </Dialog>
    </>
  );
}
