import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Loader2, SearchCheck } from "lucide-react";
import {
  solutionCriticApi,
  solutionCriticQueryKeys,
  type CompiledContextReadResponse,
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
import { buildCritiqueDigest, formatCritiqueEnum } from "./critiqueDigest";
import { SolutionCritiqueDetails } from "./SolutionCritiqueDetails";

interface SolutionCritiqueActionProps {
  sessionId: string | null | undefined;
  target: SolutionCritiqueTargetInput;
  label?: string;
  className?: string;
  size?: "sm" | "xs";
  align?: "start" | "center" | "end";
}

function humanize(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export function SolutionCritiqueAction({
  sessionId,
  target,
  label = "Critique this",
  className,
  size = "sm",
  align = "end",
}: SolutionCritiqueActionProps) {
  const [open, setOpen] = useState(false);
  const [result, setResult] = useState<SolutionCritiqueReadResponse | null>(null);
  const [context, setContext] = useState<CompiledContextReadResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const targetQueryKey = useMemo(
    () => solutionCriticQueryKeys.targetCritique(sessionId, target),
    [sessionId, target],
  );
  const targetContextQueryKey = useMemo(
    () => solutionCriticQueryKeys.targetContext(sessionId, target),
    [sessionId, target],
  );

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
      setError(err instanceof Error ? err.message : "Failed to run solution critique");
    },
  });

  if (!sessionId) return null;

  const isCompact = size === "xs";
  const activeContext = context ?? latestContextQuery.data ?? null;
  const activeResult = result ?? latestCritiqueQuery.data ?? null;
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
  const targetLabel = target.label ?? humanize(target.targetType);
  const buttonLabel = isLoading ? "Critiquing" : activeResult ? digest.pillLabel : label;

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
          />
        </DialogContent>
      </Dialog>
    </>
  );
}
