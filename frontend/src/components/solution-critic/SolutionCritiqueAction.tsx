import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, Loader2, SearchCheck } from "lucide-react";
import {
  solutionCriticApi,
  type SolutionCritiqueReadResponse,
  type SolutionCritiqueTargetInput,
} from "@/api/solution-critic";
import { Button } from "@/components/ui/button";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";

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

function critiqueItems(result: SolutionCritiqueReadResponse | null) {
  if (!result) return [];
  const critique = result.solutionCritique;
  const flagged = critique.claims.filter((claim) =>
    ["unsupported", "contradicted", "unclear"].includes(claim.status)
  );
  if (flagged.length > 0) return flagged.slice(0, 3);
  return critique.claims.slice(0, 3);
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
  const [error, setError] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const items = useMemo(() => critiqueItems(result), [result]);

  const mutation = useMutation({
    mutationFn: async () => {
      if (!sessionId) {
        throw new Error("No ideation session is available for this critique target.");
      }
      const context = await solutionCriticApi.compileTargetContext(sessionId, target);
      return solutionCriticApi.critiqueTarget(sessionId, target, context.artifactId);
    },
    onMutate: () => {
      setError(null);
      setResult(null);
      setOpen(true);
    },
    onSuccess: (response) => {
      setResult(response);
      void queryClient.invalidateQueries({ queryKey: ["solutionCritic", sessionId] });
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : "Failed to run solution critique");
    },
  });

  if (!sessionId) return null;

  const isCompact = size === "xs";
  const critique = result?.solutionCritique;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className={cn(
            "gap-1.5 rounded-md text-text-primary/55 hover:bg-[var(--overlay-moderate)] hover:text-text-primary/85",
            isCompact && "h-6 px-1.5 py-0 text-[10px]",
            className,
          )}
          onClick={(event) => {
            event.stopPropagation();
            mutation.mutate();
          }}
          disabled={mutation.isPending}
          data-testid="solution-critique-action"
          aria-label={label}
        >
          {mutation.isPending ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <SearchCheck className="h-3.5 w-3.5" />
          )}
          <span>{mutation.isPending ? "Critiquing" : label}</span>
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align={align}
        className="w-[360px] border-[var(--border-subtle)] bg-[var(--bg-surface)] p-0 text-text-primary shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="border-b border-[var(--overlay-weak)] px-3 py-2">
          <div className="text-[11px] font-semibold uppercase tracking-wide text-text-primary/40">
            Solution Critique
          </div>
          <div className="mt-0.5 truncate text-[13px] font-medium text-text-primary/85">
            {target.label ?? humanize(target.targetType)}
          </div>
        </div>

        <div className="space-y-3 px-3 py-3">
          {mutation.isPending && (
            <div className="flex items-center gap-2 text-[12px] text-text-primary/55">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              Running critique
            </div>
          )}

          {error && (
            <div className="flex items-start gap-2 rounded-md border border-[var(--status-error-border)] bg-[var(--status-error-muted)] p-2 text-[12px] text-[var(--status-error)]">
              <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <span>{error}</span>
            </div>
          )}

          {critique && (
            <>
              <div className="flex flex-wrap items-center gap-2">
                <span className="rounded-md bg-[var(--overlay-faint)] px-2 py-1 text-[11px] font-medium text-text-primary/70">
                  {humanize(critique.verdict)}
                </span>
                <span className="text-[11px] text-text-primary/40">
                  {humanize(critique.confidence)} confidence
                </span>
                <span className="text-[11px] text-text-primary/40">
                  {result.projectedGaps.length} projected gap{result.projectedGaps.length === 1 ? "" : "s"}
                </span>
              </div>

              {items.length > 0 && (
                <div className="space-y-1.5">
                  {items.map((item) => (
                    <div key={item.id} className="rounded-md bg-[var(--overlay-faint)] p-2">
                      <div className="text-[11px] font-medium uppercase tracking-wide text-text-primary/40">
                        {humanize(item.status)}
                      </div>
                      <div className="mt-1 text-[12px] leading-snug text-text-primary/75">
                        {item.claim}
                      </div>
                      {item.notes && (
                        <div className="mt-1 text-[11px] leading-snug text-text-primary/45">
                          {item.notes}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {critique.safeNextAction && (
                <div className="rounded-md border border-[var(--overlay-weak)] p-2">
                  <div className="text-[11px] font-medium uppercase tracking-wide text-text-primary/40">
                    Safe Next Action
                  </div>
                  <div className="mt-1 text-[12px] leading-snug text-text-primary/70">
                    {critique.safeNextAction}
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
